"use client";

import { useState, useCallback, useRef, useEffect } from "react";
import { getPdfjsLib, type PDFDocumentProxy } from "@/lib/pdf/pdfjs-init";
import toast from "react-hot-toast";

import {
  inspectPdfDocument,
  exportPdfDocument,
} from "@/lib/pdf/pdf-engine";
import {
  type DocumentInspectionResult,
  type ExportMode,
  type AcroFormField,
} from "@/lib/pdf/pdf-types";
import { StarPdfClient, type StarPdfDocumentHandle } from "@/lib/pdf/starpdf-client";
import {
  runStarPdfPageOperation,
  mergeStarPdfDocuments,
  type StarPdfPageOperation,
} from "@/lib/pdf/starpdf-page-worker-client";
import type {
  StarPdfImageInfo,
  StarPdfSearchResult,
  StarPdfSecurityInfo,
  StarPdfTextSpan,
  StarPdfUpdateVectorGraphicInput,
  StarPdfVectorGraphicInfo,
} from "@/lib/pdf/starpdf-types";
import { formatPdfErrorMessage } from "@/lib/pdf/pdf-friendly-errors";
import { ShieldCheck } from "lucide-react";

import { PdfDropzone } from "./PdfDropzone";
import { PdfToolbar } from "./PdfToolbar";
import { PdfThumbnailRail } from "./PdfThumbnailRail";
import { PdfPageCanvas } from "./PdfPageCanvas";
import { PdfDocumentInfo } from "./PdfDocumentInfo";
import { PdfPageOperations } from "./PdfPageOperations";
import { PdfContextualToolbar, type SelectedItem } from "./PdfContextualToolbar";
import { PdfConfirmDialog } from "./PdfConfirmDialog";

interface HistoryEntry {
  bytes: Uint8Array;
  description: string;
}

export function SmartPdfEditor() {
  const [sourceBytes, setSourceBytes] = useState<Uint8Array | null>(null);
  const [filename, setFilename] = useState<string>("");
  const [inspectionResult, setInspectionResult] = useState<DocumentInspectionResult | null>(null);
  const [pdfProxy, setPdfProxy] = useState<PDFDocumentProxy | null>(null);
  const [starPdfDoc, setStarPdfDoc] = useState<StarPdfDocumentHandle | null>(null);
  const [securityInfo, setSecurityInfo] = useState<StarPdfSecurityInfo | null>(null);

  const [currentPage, setCurrentPage] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [fieldValues, setFieldValues] = useState<Record<string, string | boolean | string[]>>({});
  const [annotationValues, setAnnotationValues] = useState<Record<string, string>>({});
  const [pageTextSpans, setPageTextSpans] = useState<StarPdfTextSpan[]>([]);
  const [pageImages, setPageImages] = useState<StarPdfImageInfo[]>([]);
  const [pageGraphics, setPageGraphics] = useState<StarPdfVectorGraphicInfo[]>([]);
  const [isModified, setIsModified] = useState<boolean>(false);
  const [selectedPages, setSelectedPages] = useState<Set<number>>(() => new Set([1]));
  const [isPageProcessing, setIsPageProcessing] = useState<boolean>(false);

  // Selection state on canvas
  const [selectedItem, setSelectedItem] = useState<SelectedItem | null>(null);

  // Operation history stack (bounded to 25 snapshots)
  const historyRef = useRef<HistoryEntry[]>([]);
  const historyIndexRef = useRef<number>(-1);
  const [historyLength, setHistoryLength] = useState<number>(0);
  const [historyIndex, setHistoryIndex] = useState<number>(-1);

  // Unsaved changes confirmation
  const [showConfirmOpenModal, setShowConfirmOpenModal] = useState<boolean>(false);

  // Layout Panel Visibility (Desktop Workspace)
  const [isThumbnailsOpen, setIsThumbnailsOpen] = useState<boolean>(true);

  // StarPDF search state
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [searchResults, setSearchResults] = useState<StarPdfSearchResult[]>([]);
  const [activeSearchIndex, setActiveSearchIndex] = useState<number>(0);

  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [loadingMessage, setLoadingMessage] = useState<string>("Loading document...");
  const [isExporting, setIsExporting] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [showInfoModal, setShowInfoModal] = useState<boolean>(false);

  const viewportContainerRef = useRef<HTMLDivElement | null>(null);
  const mergeInputRef = useRef<HTMLInputElement | null>(null);
  const pdfProxyRef = useRef<PDFDocumentProxy | null>(null);
  const starPdfDocRef = useRef<StarPdfDocumentHandle | null>(null);

  useEffect(() => {
    pdfProxyRef.current = pdfProxy;
  }, [pdfProxy]);

  useEffect(() => {
    starPdfDocRef.current = starPdfDoc;
  }, [starPdfDoc]);

  // Clean up PDF.js proxy and StarPDF handle when unmounting or resetting
  const cleanupProxy = useCallback(() => {
    if (pdfProxyRef.current) {
      void pdfProxyRef.current.destroy();
      pdfProxyRef.current = null;
      setPdfProxy(null);
    }
    if (starPdfDocRef.current) {
      void starPdfDocRef.current.close();
      starPdfDocRef.current = null;
      setStarPdfDoc(null);
    }
  }, []);

  useEffect(() => {
    return () => {
      cleanupProxy();
    };
  }, [cleanupProxy]);

  const pushHistorySnapshot = useCallback((newBytes: Uint8Array, description: string) => {
    const currentIdx = historyIndexRef.current;
    const sliced = historyRef.current.slice(0, currentIdx + 1);
    sliced.push({ bytes: newBytes, description });
    if (sliced.length > 25) sliced.shift();
    historyRef.current = sliced;
    const nextIdx = sliced.length - 1;
    historyIndexRef.current = nextIdx;
    setHistoryLength(sliced.length);
    setHistoryIndex(nextIdx);
    setIsModified(true);
  }, []);

  const loadDocument = useCallback(
    async (bytes: Uint8Array, docFilename: string, docSize: number, initialPage = 1, isHistoryRestore = false) => {
      setIsLoading(true);
      setError(null);
      setSecurityInfo(null);
      setSelectedItem(null);
      setLoadingMessage("Parsing PDF structure & inspecting page objects...");

      try {
        // 1. Open StarPDF first so security policy is established
        let starDoc: StarPdfDocumentHandle | null = null;
        try {
          starDoc = await StarPdfClient.open(bytes);
          const detectedSecurity = await starDoc.getSecurityInfo();
          setSecurityInfo(detectedSecurity);
          if (detectedSecurity.encryption_state !== "NOT_ENCRYPTED") {
            await starDoc.close();
            const friendly = formatPdfErrorMessage("STANDARD_SECURITY_DETECTED");
            setError(friendly.userMessage);
            setIsLoading(false);
            toast.error(friendly.userMessage);
            return;
          }
        } catch (starErr) {
          console.warn("StarPDF validation note:", starErr);
        }

        // 2. Inspect using pdf-lib (AcroForms, metadata, dimensions)
        const inspected = await inspectPdfDocument(bytes, docFilename, docSize);

        // 3. Load into PDF.js for canvas rendering
        setLoadingMessage("Initializing document viewer...");
        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: bytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        // 4. Clean up previous handles now that new proxy is ready
        if (pdfProxyRef.current && pdfProxyRef.current !== proxy) {
          void pdfProxyRef.current.destroy();
        }
        if (starPdfDocRef.current && starPdfDocRef.current !== starDoc) {
          void starPdfDocRef.current.close();
        }

        // 5. Initialize field values dictionary
        const initialValues: Record<string, string | boolean | string[]> = {};
        for (const field of inspected.fields) {
          initialValues[field.name] = field.value;
        }

        const initialAnnotationValues: Record<string, string> = {};
        for (const annot of inspected.annotations || []) {
          initialAnnotationValues[annot.id] = annot.contents;
        }

        setSourceBytes(bytes);
        setFilename(docFilename);
        setInspectionResult(inspected);
        setStarPdfDoc(starDoc);
        setPdfProxy(proxy);
        const validPage = Math.max(1, Math.min(initialPage, inspected.metadata.pageCount || 1));
        setCurrentPage(validPage);
        setScale(1.0);
        setFieldValues(initialValues);
        setAnnotationValues(initialAnnotationValues);
        setSelectedPages(new Set([validPage]));
        setSearchQuery("");
        setSearchResults([]);
        setActiveSearchIndex(0);
        setIsLoading(false);

        if (!isHistoryRestore) {
          setIsModified(false);
          historyRef.current = [{ bytes, description: "Initial document" }];
          historyIndexRef.current = 0;
          setHistoryLength(1);
          setHistoryIndex(0);
          if (inspected.fields.length > 0) {
            toast.success(`Loaded "${docFilename}" with ${inspected.fields.length} interactive form field(s).`);
          } else {
            toast.success(`Loaded "${docFilename}" (${inspected.metadata.pageCount} pages).`);
          }
        }
      } catch (err: unknown) {
        setIsLoading(false);
        cleanupProxy();
        const friendly = formatPdfErrorMessage(err);
        setError(friendly.userMessage);
        toast.error(friendly.userMessage);
      }
    },
    [cleanupProxy],
  );

  const handleUndo = useCallback(async () => {
    const currentIdx = historyIndexRef.current;
    if (currentIdx > 0) {
      const prevIdx = currentIdx - 1;
      const targetEntry = historyRef.current[prevIdx];
      historyIndexRef.current = prevIdx;
      setHistoryIndex(prevIdx);
      await loadDocument(targetEntry.bytes, filename, targetEntry.bytes.byteLength, currentPage, true);
      toast.success(`Undo: ${historyRef.current[currentIdx].description}`);
    }
  }, [currentPage, filename, loadDocument]);

  const handleRedo = useCallback(async () => {
    const currentIdx = historyIndexRef.current;
    if (currentIdx < historyRef.current.length - 1) {
      const nextIdx = currentIdx + 1;
      const targetEntry = historyRef.current[nextIdx];
      historyIndexRef.current = nextIdx;
      setHistoryIndex(nextIdx);
      await loadDocument(targetEntry.bytes, filename, targetEntry.bytes.byteLength, currentPage, true);
      toast.success(`Redo: ${targetEntry.description}`);
    }
  }, [currentPage, filename, loadDocument]);

  // Global keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        if (e.shiftKey) {
          e.preventDefault();
          void handleRedo();
        } else {
          e.preventDefault();
          void handleUndo();
        }
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        void handleRedo();
      } else if (e.key === "Escape") {
        setSelectedItem(null);
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleUndo, handleRedo]);

  const handleFieldValueChange = useCallback(
    (name: string, value: string | boolean | string[]) => {
      setFieldValues((prev) => ({
        ...prev,
        [name]: value,
      }));
      setIsModified(true);
    },
    [],
  );

  const _handleResetForm = useCallback(() => {
    if (!inspectionResult) return;
    const originalValues: Record<string, string | boolean | string[]> = {};
    for (const field of inspectionResult.fields) {
      originalValues[field.name] = Array.isArray(field.originalValue)
        ? [...field.originalValue]
        : field.originalValue;
    }
    setFieldValues(originalValues);
    setIsModified(false);
    toast.success("Form values restored to original document state.");
  }, [inspectionResult]);

  const handleZoomIn = useCallback(() => {
    setScale((prev) => Math.min(Math.round((prev + 0.15) * 100) / 100, 4.0));
  }, []);

  const handleZoomOut = useCallback(() => {
    setScale((prev) => Math.max(Math.round((prev - 0.15) * 100) / 100, 0.25));
  }, []);

  const handleFitWidth = useCallback(() => {
    if (!viewportContainerRef.current || !inspectionResult) return;
    const containerWidth = viewportContainerRef.current.clientWidth - 64;
    const pageInfo = inspectionResult.pages[currentPage - 1];
    if (pageInfo && pageInfo.width > 0) {
      const newScale = Math.min(Math.max(containerWidth / pageInfo.width, 0.25), 4.0);
      setScale(Math.round(newScale * 100) / 100);
    }
  }, [inspectionResult, currentPage]);

  const handleFitPage = useCallback(() => {
    if (!viewportContainerRef.current || !inspectionResult) return;
    const containerHeight = viewportContainerRef.current.clientHeight - 64;
    const pageInfo = inspectionResult.pages[currentPage - 1];
    if (pageInfo && pageInfo.height > 0) {
      const newScale = Math.min(Math.max(containerHeight / pageInfo.height, 0.25), 4.0);
      setScale(Math.round(newScale * 100) / 100);
    }
  }, [inspectionResult, currentPage]);

  const handleSearchChange = useCallback(
    async (query: string) => {
      setSearchQuery(query);
      if (!starPdfDoc || !query.trim()) {
        setSearchResults([]);
        setActiveSearchIndex(0);
        return;
      }

      try {
        const hits = await starPdfDoc.search(query, { caseSensitive: false });
        setSearchResults(hits);
        setActiveSearchIndex(0);
        if (hits.length > 0) {
          setCurrentPage(hits[0].page_index + 1);
        }
      } catch (err) {
        console.error("StarPDF search failed:", err);
      }
    },
    [starPdfDoc],
  );

  const handleNextSearchResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const nextIdx = (activeSearchIndex + 1) % searchResults.length;
    setActiveSearchIndex(nextIdx);
    setCurrentPage(searchResults[nextIdx].page_index + 1);
  }, [searchResults, activeSearchIndex]);

  const handlePrevSearchResult = useCallback(() => {
    if (searchResults.length === 0) return;
    const prevIdx = (activeSearchIndex - 1 + searchResults.length) % searchResults.length;
    setActiveSearchIndex(prevIdx);
    setCurrentPage(searchResults[prevIdx].page_index + 1);
  }, [searchResults, activeSearchIndex]);

  // Synchronize text spans, images, and graphics on page change
  useEffect(() => {
    let cancelled = false;
    if (!starPdfDoc) return;
    void starPdfDoc
      .extractPageText(currentPage - 1)
      .then((pageText) => {
        if (!cancelled) {
          setPageTextSpans(pageText.spans || []);
        }
      })
      .catch((err) => {
        console.warn("Failed to extract page text:", err);
        if (!cancelled) setPageTextSpans([]);
      });

    void starPdfDoc
      .enumerateImages(currentPage - 1)
      .then((images) => {
        if (!cancelled) {
          setPageImages(images || []);
        }
      })
      .catch((err) => {
        console.warn("Failed to enumerate page images:", err);
        if (!cancelled) setPageImages([]);
      });

    void starPdfDoc
      .enumerateGraphics(currentPage - 1)
      .then((graphics) => {
        if (!cancelled) {
          setPageGraphics(graphics || []);
        }
      })
      .catch((err) => {
        console.warn("Failed to enumerate page graphics:", err);
        if (!cancelled) setPageGraphics([]);
      });

    return () => {
      cancelled = true;
    };
  }, [starPdfDoc, currentPage, sourceBytes]);

  const handleReplaceExistingText = useCallback(
    async (spanId: string, newText: string) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        const result = await starPdfDoc.replaceText(currentPage - 1, spanId, newText);
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, `Edit text "${newText}"`);

        const pageText = await starPdfDoc.extractPageText(currentPage - 1);
        setPageTextSpans(pageText.spans || []);

        toast.success(`Text updated (${result.layout_result}). Native content stream modified.`);
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const handleReplaceImage = useCallback(
    async (imageId: string, file: File) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        await starPdfDoc.replaceImage(currentPage - 1, imageId, bytes, true);
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Replace image");

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image replaced successfully.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const _handleAddImage = useCallback(
    async (file: File, x: number, y: number, width: number, height: number) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);
        await starPdfDoc.addImage(currentPage - 1, bytes, x, y, width, height);
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Add image");

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image added to page successfully.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const handleRemoveImage = useCallback(
    async (imageId: string) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        await starPdfDoc.removeImage(currentPage - 1, imageId);
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Remove image");
        setSelectedItem(null);

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image removed from page.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const handleUpdateGraphic = useCallback(
    async (input: StarPdfUpdateVectorGraphicInput) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        await starPdfDoc.updateGraphic(input);
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Update vector shape");

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Vector shape updated successfully.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const _handleAddRectangle = useCallback(
    async (
      x: number,
      y: number,
      width: number,
      height: number,
      strokeColorHex?: string,
      fillColorHex?: string,
      lineWidth = 1.5,
    ) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        const hexToRgb = (hex: string): [number, number, number] => {
          const clean = hex.replace("#", "");
          if (clean.length === 6) {
            return [
              parseInt(clean.substring(0, 2), 16) / 255,
              parseInt(clean.substring(2, 4), 16) / 255,
              parseInt(clean.substring(4, 6), 16) / 255,
            ];
          }
          return [0, 0, 0];
        };

        const strokeRgb = strokeColorHex ? hexToRgb(strokeColorHex) : undefined;
        const fillRgb = fillColorHex ? hexToRgb(fillColorHex) : undefined;

        await starPdfDoc.addRectangle({
          page_index: currentPage - 1,
          x,
          y,
          width,
          height,
          stroke_color_rgb: strokeRgb,
          fill_color_rgb: fillRgb,
          line_width: lineWidth,
          is_stroked: Boolean(strokeRgb),
          is_filled: Boolean(fillRgb),
        });

        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Add rectangle");

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Rectangle added successfully.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const _handleAddLine = useCallback(
    async (
      x1: number,
      y1: number,
      x2: number,
      y2: number,
      strokeColorHex = "#000000",
      lineWidth = 2.0,
    ) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        const hexToRgb = (hex: string): [number, number, number] => {
          const clean = hex.replace("#", "");
          if (clean.length === 6) {
            return [
              parseInt(clean.substring(0, 2), 16) / 255,
              parseInt(clean.substring(2, 4), 16) / 255,
              parseInt(clean.substring(4, 6), 16) / 255,
            ];
          }
          return [0, 0, 0];
        };

        const strokeRgb = hexToRgb(strokeColorHex);

        await starPdfDoc.addLine({
          page_index: currentPage - 1,
          x1,
          y1,
          x2,
          y2,
          stroke_color_rgb: strokeRgb,
          line_width: lineWidth,
        });

        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Add line");

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Line added successfully.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const handleDeleteGraphic = useCallback(
    async (graphicId: string) => {
      if (!starPdfDoc || !sourceBytes) return;
      try {
        await starPdfDoc.deleteGraphic({
          page_index: currentPage - 1,
          graphic_id: graphicId,
          clone_if_shared: true,
        });
        const updatedBytes = await starPdfDoc.exportIncremental();

        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: updatedBytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        if (pdfProxy) {
          void pdfProxy.destroy();
        }
        setPdfProxy(proxy);
        setSourceBytes(updatedBytes);
        pushHistorySnapshot(updatedBytes, "Delete shape");
        setSelectedItem(null);

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Vector shape removed from page.");
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy, pushHistorySnapshot],
  );

  const handleAnnotationChange = useCallback(
    (annotId: string, value: string) => {
      setAnnotationValues((prev) => ({
        ...prev,
        [annotId]: value,
      }));
      setIsModified(true);
      if (sourceBytes) {
        pushHistorySnapshot(sourceBytes, "Edit annotation");
      }
    },
    [sourceBytes, pushHistorySnapshot],
  );

  const handleExport = useCallback(
    async (mode: ExportMode) => {
      if (!sourceBytes || !inspectionResult) return;
      setIsExporting(true);

      try {
        const result = await exportPdfDocument(
          sourceBytes,
          filename,
          fieldValues,
          mode,
          inspectionResult.metadata.pageCount,
          annotationValues,
        );

        // Create browser download
        const blob = new Blob([new Uint8Array(result.pdfBytes)], { type: "application/pdf" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = result.filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        setIsModified(false);
        toast.success(
          mode === "editable"
            ? `Exported "${result.filename}" with interactive form fields and annotations.`
            : `Exported "${result.filename}" with flattened content.`,
        );
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      } finally {
        setIsExporting(false);
      }
    },
    [sourceBytes, filename, fieldValues, annotationValues, inspectionResult],
  );

  const downloadPdf = useCallback((bytes: Uint8Array, outputFilename: string) => {
    const blob = new Blob([new Uint8Array(bytes)], { type: "application/pdf" });
    const url = URL.createObjectURL(blob);
    const anchor = document.createElement("a");
    anchor.href = url;
    anchor.download = outputFilename;
    document.body.appendChild(anchor);
    anchor.click();
    document.body.removeChild(anchor);
    URL.revokeObjectURL(url);
  }, []);

  const applyPageOperation = useCallback(
    async (operation: StarPdfPageOperation, nextPage: number, successMessage: string) => {
      if (!sourceBytes || isPageProcessing) return;
      if (isModified && Object.keys(fieldValues).length > 0) {
        toast.error("Export or reset pending form edits before changing pages.");
        return;
      }
      setIsPageProcessing(true);
      try {
        const output = await runStarPdfPageOperation(sourceBytes, operation);
        await loadDocument(output, filename, output.byteLength, nextPage, true);
        pushHistorySnapshot(output, successMessage);
        toast.success(successMessage);
      } catch (operationError) {
        const friendly = formatPdfErrorMessage(operationError);
        setError(friendly.userMessage);
        toast.error(friendly.userMessage);
      } finally {
        setIsPageProcessing(false);
      }
    },
    [fieldValues, filename, isModified, isPageProcessing, loadDocument, pushHistorySnapshot, sourceBytes],
  );

  const handleExtractPages = useCallback(async () => {
    if (!sourceBytes || isPageProcessing || selectedPages.size === 0) return;
    if (isModified && Object.keys(fieldValues).length > 0) {
      toast.error("Export or reset pending form edits before extracting pages.");
      return;
    }
    setIsPageProcessing(true);
    try {
      const pageIndices = Array.from(selectedPages)
        .sort((left, right) => left - right)
        .map((pageNumber) => pageNumber - 1);
      const output = await runStarPdfPageOperation(sourceBytes, {
        type: "extractPages",
        pageIndices,
      });
      const baseName = filename.replace(/\.pdf$/i, "");
      downloadPdf(output, `${baseName}-extracted.pdf`);
      toast.success(`Extracted ${pageIndices.length} page(s) as a standalone PDF.`);
    } catch (operationError) {
      const friendly = formatPdfErrorMessage(operationError);
      setError(friendly.userMessage);
      toast.error(friendly.userMessage);
    } finally {
      setIsPageProcessing(false);
    }
  }, [downloadPdf, fieldValues, filename, isModified, isPageProcessing, selectedPages, sourceBytes]);

  const handleMergeFiles = useCallback(
    async (files: FileList | null) => {
      if (!sourceBytes || !files?.length || isPageProcessing) return;
      if (isModified && Object.keys(fieldValues).length > 0) {
        toast.error("Export or reset pending form edits before adding another PDF.");
        return;
      }
      setIsPageProcessing(true);
      try {
        const additions = await Promise.all(
          Array.from(files).map(async (file) => new Uint8Array(await file.arrayBuffer())),
        );
        const output = await mergeStarPdfDocuments([sourceBytes, ...additions]);
        cleanupProxy();
        await loadDocument(output, filename, output.byteLength, 1, true);
        pushHistorySnapshot(output, `Merged ${additions.length} PDF(s)`);
        toast.success(`Added and merged ${additions.length} PDF document(s).`);
      } catch (operationError) {
        const friendly = formatPdfErrorMessage(operationError);
        setError(friendly.userMessage);
        toast.error(friendly.userMessage);
      } finally {
        if (mergeInputRef.current) mergeInputRef.current.value = "";
        setIsPageProcessing(false);
      }
    },
    [cleanupProxy, fieldValues, filename, isModified, isPageProcessing, loadDocument, pushHistorySnapshot, sourceBytes],
  );

  const handleTogglePageSelection = useCallback((pageNumber: number) => {
    setSelectedPages((previous) => {
      const next = new Set(previous);
      if (next.has(pageNumber)) next.delete(pageNumber);
      else next.add(pageNumber);
      return next;
    });
  }, []);

  const performOpenNewFile = useCallback(() => {
    cleanupProxy();
    setSourceBytes(null);
    setFilename("");
    setInspectionResult(null);
    setFieldValues({});
    setIsModified(false);
    setSelectedPages(new Set([1]));
    setError(null);
    setSecurityInfo(null);
    setCurrentPage(1);
    setScale(1.0);
    setSearchQuery("");
    setSearchResults([]);
    historyRef.current = [];
    historyIndexRef.current = -1;
    setHistoryLength(0);
    setHistoryIndex(-1);
    setSelectedItem(null);
  }, [cleanupProxy]);

  const handleOpenNewFileClick = useCallback(() => {
    if (isModified) {
      setShowConfirmOpenModal(true);
    } else {
      performOpenNewFile();
    }
  }, [isModified, performOpenNewFile]);

  // Dropzone screen when no document is active
  if (!sourceBytes || !inspectionResult) {
    return (
      <div
        className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
        data-testid="smartpdf-editor-workspace"
      >
        {/* Top Header for Empty Application State */}
        <header className="h-14 border-b border-slate-200 bg-white px-4 sm:px-6 flex items-center justify-between shrink-0 select-none">
          <div className="flex items-center gap-3">
            <div className="grid size-8 place-items-center rounded-xl bg-gradient-to-br from-sky-500 to-indigo-600 text-white font-bold text-xs shadow-xs">
              SP
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-semibold text-slate-900 text-sm tracking-tight">SmartPDF</span>
                <span className="text-[10px] font-medium px-2 py-0.5 rounded-full bg-slate-100 text-slate-600 border border-slate-200">
                  Powered by StarPDF
                </span>
              </div>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <span className="inline-flex items-center gap-1 text-xs text-slate-500 bg-slate-50 border border-slate-200 px-2.5 py-1 rounded-full">
              <ShieldCheck className="size-3.5 text-emerald-600" />
              Local processing
            </span>
          </div>
        </header>

        {/* Center Empty Workspace with Dropzone */}
        <div className="flex-1 flex items-center justify-center p-6 sm:p-12 overflow-auto bg-slate-50/50">
          <div className="w-full max-w-xl space-y-6">
            <PdfDropzone
              onFileSelect={loadDocument}
              isLoading={isLoading}
              loadingMessage={loadingMessage}
              error={error}
            />
          </div>
        </div>

        {/* Bottom Status Bar for Empty State */}
        <footer
          className="h-8 border-t border-slate-200 bg-white px-4 flex items-center justify-between text-xs text-slate-400 shrink-0 select-none"
          data-testid="smartpdf-status-bar"
        >
          <span>No document open</span>
          <span className="flex items-center gap-1.5 text-slate-500">
            <ShieldCheck className="size-3.5 text-emerald-600" />
            Zero bytes uploaded • 100% Client-Side WebAssembly
          </span>
        </footer>
      </div>
    );
  }

  const currentPageInfo = inspectionResult.pages[currentPage - 1] || {
    width: 612,
    height: 792,
    rotation: 0,
  };

  return (
    <div
      className="flex flex-col h-full w-full bg-slate-100 overflow-hidden relative select-none"
      data-testid="smartpdf-editor-workspace"
    >
      {securityInfo && securityInfo.signature_state !== "UNSIGNED" ? (
        <div
          className="border-b border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950 flex items-center justify-between shrink-0"
          data-testid="starpdf-signed-document-warning"
          role="status"
        >
          <span>
            This PDF contains a digital signature. StarPDF preserves original signed bytes when appending updates,
            but does not verify cryptographic signature validity for post-signature revision.
          </span>
        </div>
      ) : null}

      {/* Top Application Toolbar */}
      <PdfToolbar
        filename={filename}
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        scale={scale}
        isExporting={isExporting}
        isModified={isModified}
        canUndo={historyIndex > 0}
        canRedo={historyIndex < historyLength - 1}
        onUndo={() => void handleUndo()}
        onRedo={() => void handleRedo()}
        searchQuery={searchQuery}
        searchResultCount={searchResults.length}
        activeSearchIndex={activeSearchIndex}
        onSearchChange={(q) => void handleSearchChange(q)}
        onNextSearchResult={handleNextSearchResult}
        onPrevSearchResult={handlePrevSearchResult}
        onPageChange={setCurrentPage}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onFitWidth={handleFitWidth}
        onFitPage={handleFitPage}
        onExport={handleExport}
        onShowInfo={() => setShowInfoModal(true)}
        onOpenNewFile={handleOpenNewFileClick}
        onMergeClick={() => mergeInputRef.current?.click()}
        isThumbnailsOpen={isThumbnailsOpen}
        onToggleThumbnails={() => setIsThumbnailsOpen(!isThumbnailsOpen)}
      />

      {/* Page Operations Rail / Actions */}
      <PdfPageOperations
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        selectedCount={selectedPages.size}
        isProcessing={isPageProcessing}
        onDelete={() => {
          if (inspectionResult.metadata.pageCount <= 1) {
            toast.error("Cannot delete the only page in a document.");
            return;
          }
          const targetNextPage =
            currentPage >= inspectionResult.metadata.pageCount
              ? Math.max(1, inspectionResult.metadata.pageCount - 1)
              : currentPage;

          void applyPageOperation(
            { type: "deletePage", pageIndex: currentPage - 1 },
            targetNextPage,
            "Page deleted.",
          );
        }}
        onMoveLeft={() =>
          void applyPageOperation(
            { type: "movePage", fromIndex: currentPage - 1, toIndex: currentPage - 2 },
            currentPage - 1,
            "Page moved left.",
          )
        }
        onMoveRight={() =>
          void applyPageOperation(
            { type: "movePage", fromIndex: currentPage - 1, toIndex: currentPage },
            currentPage + 1,
            "Page moved right.",
          )
        }
        onDuplicate={() =>
          void applyPageOperation(
            {
              type: "duplicatePage",
              pageIndex: currentPage - 1,
              destinationIndex: currentPage,
            },
            currentPage + 1,
            "Page duplicated.",
          )
        }
        onInsertBlank={() =>
          void applyPageOperation(
            {
              type: "insertBlankPage",
              pageIndex: currentPage,
              width: currentPageInfo.width,
              height: currentPageInfo.height,
              rotation: 0,
            },
            currentPage + 1,
            "Blank page inserted.",
          )
        }
        onExtract={() => void handleExtractPages()}
        onMerge={() => mergeInputRef.current?.click()}
      />

      {/* Hidden File Input for Add PDF / Merge Workflow */}
      <input
        ref={mergeInputRef}
        type="file"
        accept="application/pdf,.pdf"
        multiple
        className="hidden"
        aria-label="Add PDF documents"
        onChange={(event) => void handleMergeFiles(event.target.files)}
      />

      {/* Main Workspace: Thumbnails + Viewport Canvas */}
      <div className="flex-1 flex overflow-hidden relative min-h-0">
        {/* Left Thumbnail Rail */}
        {isThumbnailsOpen && (
          <PdfThumbnailRail
            pdfDocument={pdfProxy}
            pageCount={inspectionResult.metadata.pageCount}
            currentPage={currentPage}
            onPageSelect={(p) => {
              setCurrentPage(p);
              setSelectedItem(null);
            }}
            selectedPages={selectedPages}
            onToggleSelection={handleTogglePageSelection}
            className="flex"
          />
        )}

        {/* Center Viewport Canvas Surface */}
        <main
          ref={viewportContainerRef}
          className="flex-1 bg-slate-100/90 overflow-auto p-6 sm:p-8 flex items-center justify-center min-w-0 relative"
          aria-label="PDF Document Page Viewport"
          onClick={() => setSelectedItem(null)}
        >
          {/* Contextual Action Bar */}
          <PdfContextualToolbar
            selection={selectedItem}
            onDeselect={() => setSelectedItem(null)}
            onReplaceText={handleReplaceExistingText}
            onReplaceImage={handleReplaceImage}
            onRemoveImage={handleRemoveImage}
            onUpdateGraphic={handleUpdateGraphic}
            onDeleteGraphic={handleDeleteGraphic}
            onFormFieldChange={handleFieldValueChange}
            formFieldValue={
              selectedItem?.type === "form"
                ? fieldValues[(selectedItem.data as AcroFormField).name]
                : undefined
            }
            onAnnotationChange={handleAnnotationChange}
            annotationValue={
              selectedItem?.type === "annotation"
                ? annotationValues[selectedItem.id]
                : undefined
            }
          />

          <div className="my-auto transition-transform duration-75">
            <PdfPageCanvas
              pdfDocument={pdfProxy}
              pageNumber={currentPage}
              scale={scale}
              rotation={currentPageInfo.rotation}
              pageWidth={currentPageInfo.width}
              pageHeight={currentPageInfo.height}
              textSpans={pageTextSpans}
              images={pageImages}
              graphics={pageGraphics}
              fields={inspectionResult.fields}
              annotations={inspectionResult.annotations}
              selectedItem={selectedItem}
              onSelectItem={setSelectedItem}
            />
          </div>
        </main>
      </div>

      {/* Bottom Status Bar */}
      <footer
        className="h-8 border-t border-slate-200 bg-white px-4 flex items-center justify-between text-xs text-slate-500 shrink-0 select-none"
        data-testid="smartpdf-status-bar"
      >
        <div className="flex items-center gap-3">
          <span>Page {currentPage} / {inspectionResult.metadata.pageCount}</span>
          <span className="text-slate-300">•</span>
          <span>Zoom {Math.round(scale * 100)}%</span>
          {isModified && (
            <>
              <span className="text-slate-300">•</span>
              <span className="text-amber-600 font-medium">Unsaved changes</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-3">
          {selectedItem ? (
            <span className="text-sky-700 font-medium bg-sky-50 px-2 py-0.5 rounded border border-sky-200/60">
              Selected: {selectedItem.type.toUpperCase()}
            </span>
          ) : (
            <span className="text-slate-400">No selection</span>
          )}
          <span className="text-slate-300">•</span>
          <span className="flex items-center gap-1 text-slate-500">
            <ShieldCheck className="size-3.5 text-emerald-600" />
            Local processing
          </span>
        </div>
      </footer>

      {/* Document Properties Modal */}
      <PdfDocumentInfo
        metadata={inspectionResult.metadata}
        isOpen={showInfoModal}
        onClose={() => setShowInfoModal(false)}
      />

      {/* Unsaved Changes Confirmation Dialog */}
      <PdfConfirmDialog
        isOpen={showConfirmOpenModal}
        title="Discard unsaved changes?"
        description="You have unsaved edits in this document. Opening another PDF will discard these modifications."
        confirmLabel="Discard & Open"
        cancelLabel="Keep Editing"
        isDestructive={true}
        onConfirm={() => {
          setShowConfirmOpenModal(false);
          performOpenNewFile();
        }}
        onCancel={() => setShowConfirmOpenModal(false)}
      />
    </div>
  );
}
