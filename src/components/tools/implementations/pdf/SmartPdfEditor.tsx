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
} from "@/lib/pdf/pdf-types";
import { PdfError } from "@/lib/pdf/pdf-errors";
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

import { PdfDropzone } from "./PdfDropzone";
import { PdfToolbar } from "./PdfToolbar";
import { PdfThumbnailRail } from "./PdfThumbnailRail";
import { PdfPageCanvas } from "./PdfPageCanvas";
import { PdfFormInspector } from "./PdfFormInspector";
import { PdfDocumentInfo } from "./PdfDocumentInfo";
import { PdfPageOperations } from "./PdfPageOperations";

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
  const [pageTextSpans, setPageTextSpans] = useState<StarPdfTextSpan[]>([]);
  const [pageImages, setPageImages] = useState<StarPdfImageInfo[]>([]);
  const [pageGraphics, setPageGraphics] = useState<StarPdfVectorGraphicInfo[]>([]);
  const [isModified, setIsModified] = useState<boolean>(false);
  const [selectedPages, setSelectedPages] = useState<Set<number>>(() => new Set([1]));
  const [isPageProcessing, setIsPageProcessing] = useState<boolean>(false);

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

  // Clean up PDF.js proxy and StarPDF handle when unmounting or resetting
  const cleanupProxy = useCallback(() => {
    if (pdfProxy) {
      void pdfProxy.destroy();
      setPdfProxy(null);
    }
    if (starPdfDoc) {
      void starPdfDoc.close();
      setStarPdfDoc(null);
    }
  }, [pdfProxy, starPdfDoc]);

  useEffect(() => {
    return () => {
      cleanupProxy();
    };
  }, [cleanupProxy]);

  const loadDocument = useCallback(
    async (bytes: Uint8Array, docFilename: string, docSize: number, initialPage = 1) => {
      setIsLoading(true);
      setError(null);
      setSecurityInfo(null);
      setLoadingMessage("Parsing PDF structure & detecting form fields...");

      try {
        // 1. Open StarPDF first so security policy is established before other parsers/editors.
        let starDoc: StarPdfDocumentHandle | null = null;
        try {
          starDoc = await StarPdfClient.open(bytes);
          const detectedSecurity = await starDoc.getSecurityInfo();
          setSecurityInfo(detectedSecurity);
          if (detectedSecurity.encryption_state !== "NOT_ENCRYPTED") {
            await starDoc.close();
            const message =
              "This PDF is encrypted with an unsupported security handler. Editing is unavailable; StarPDF does not decrypt or bypass document security.";
            setError(message);
            setIsLoading(false);
            toast.error(message);
            return;
          }
        } catch (starErr) {
          console.warn("StarPDF validation note:", starErr);
        }

        // 2. Inspect using pdf-lib (AcroForms, metadata, dimensions)
        const inspected = await inspectPdfDocument(bytes, docFilename, docSize);

        // 3. Load into PDF.js for canvas rendering (copy bytes buffer to prevent detached ArrayBuffer)
        setLoadingMessage("Initializing document viewer...");
        const pdfjsLib = await getPdfjsLib();
        const loadingTask = pdfjsLib.getDocument({
          data: bytes.slice(0),
          cMapUrl: "https://cdn.jsdelivr.net/npm/pdfjs-dist@5.6.205/cmaps/",
          cMapPacked: true,
        });
        const proxy = await loadingTask.promise;

        // 4. Initialize field values dictionary
        const initialValues: Record<string, string | boolean | string[]> = {};
        for (const field of inspected.fields) {
          initialValues[field.name] = field.value;
        }

        setSourceBytes(bytes);
        setFilename(docFilename);
        setInspectionResult(inspected);
        setStarPdfDoc(starDoc);
        setPdfProxy(proxy);
        setCurrentPage(initialPage);
        setScale(1.0);
        setFieldValues(initialValues);
        setIsModified(false);
        setSelectedPages(new Set([initialPage]));
        setSearchQuery("");
        setSearchResults([]);
        setActiveSearchIndex(0);
        setIsLoading(false);

        if (inspected.fields.length > 0) {
          toast.success(`Loaded "${docFilename}" with ${inspected.fields.length} interactive form field(s).`);
        } else {
          toast.success(`Loaded "${docFilename}" (${inspected.metadata.pageCount} pages).`);
        }
      } catch (err: unknown) {
        setIsLoading(false);
        cleanupProxy();
        if (err instanceof PdfError) {
          setError(err.message);
          toast.error(err.message);
        } else {
          const msg = err instanceof Error ? err.message : "Failed to load PDF document.";
          setError(msg);
          toast.error(msg);
        }
      }
    },
    [cleanupProxy],
  );

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

  const handleResetForm = useCallback(() => {
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

  // Synchronize text spans and images on page change
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

        // Reload PDF.js with modified bytes
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
        setIsModified(true);

        // Re-extract page text spans
        const pageText = await starPdfDoc.extractPageText(currentPage - 1);
        setPageTextSpans(pageText.spans || []);

        toast.success(`Text updated (${result.layout_result}). Native content stream modified.`);
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to replace text.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
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
        setIsModified(true);

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image replaced successfully.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to replace image.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
  );

  const handleAddImage = useCallback(
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
        setIsModified(true);

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image added to page successfully.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to add image.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
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
        setIsModified(true);

        const images = await starPdfDoc.enumerateImages(currentPage - 1);
        setPageImages(images || []);

        toast.success("Image removed from page.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to remove image.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
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
        setIsModified(true);

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Vector shape updated successfully.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to update vector shape.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
  );

  const handleAddRectangle = useCallback(
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
        setIsModified(true);

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Rectangle added successfully.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to add rectangle.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
  );

  const handleAddLine = useCallback(
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
        setIsModified(true);

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Line added successfully.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to add line.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
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
        setIsModified(true);

        const graphics = await starPdfDoc.enumerateGraphics(currentPage - 1);
        setPageGraphics(graphics || []);

        toast.success("Vector shape removed from page.");
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to delete vector shape.";
        toast.error(msg);
      }
    },
    [starPdfDoc, sourceBytes, currentPage, pdfProxy],
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
        );

        // Create browser download
        const blob = new Blob([result.pdfBytes.buffer as ArrayBuffer], { type: "application/pdf" });
        const url = URL.createObjectURL(blob);
        const a = document.createElement("a");
        a.href = url;
        a.download = result.filename;
        document.body.appendChild(a);
        a.click();
        document.body.removeChild(a);
        URL.revokeObjectURL(url);

        toast.success(
          mode === "editable"
            ? `Exported "${result.filename}" with interactive form fields.`
            : `Exported "${result.filename}" with flattened form content.`,
        );
      } catch (err: unknown) {
        const msg = err instanceof Error ? err.message : "Failed to export PDF.";
        toast.error(msg);
      } finally {
        setIsExporting(false);
      }
    },
    [sourceBytes, filename, fieldValues, inspectionResult],
  );

  const downloadPdf = useCallback((bytes: Uint8Array, outputFilename: string) => {
    const blob = new Blob([bytes.buffer as ArrayBuffer], { type: "application/pdf" });
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
      if (isModified) {
        toast.error("Export or reset pending form edits before changing pages.");
        return;
      }
      setIsPageProcessing(true);
      try {
        const output = await runStarPdfPageOperation(sourceBytes, operation);
        cleanupProxy();
        await loadDocument(output, filename, output.byteLength, nextPage);
        toast.success(successMessage);
      } catch (operationError) {
        const message =
          operationError instanceof Error
            ? operationError.message
            : "StarPDF could not complete the page operation.";
        setError(message);
        toast.error(message);
      } finally {
        setIsPageProcessing(false);
      }
    },
    [cleanupProxy, filename, isModified, isPageProcessing, loadDocument, sourceBytes],
  );

  const handleExtractPages = useCallback(async () => {
    if (!sourceBytes || isPageProcessing || selectedPages.size === 0) return;
    if (isModified) {
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
      const message =
        operationError instanceof Error
          ? operationError.message
          : "StarPDF could not extract the selected pages.";
      setError(message);
      toast.error(message);
    } finally {
      setIsPageProcessing(false);
    }
  }, [downloadPdf, filename, isModified, isPageProcessing, selectedPages, sourceBytes]);

  const handleMergeFiles = useCallback(
    async (files: FileList | null) => {
      if (!sourceBytes || !files?.length || isPageProcessing) return;
      if (isModified) {
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
        await loadDocument(output, filename, output.byteLength);
        toast.success(`Added ${additions.length} PDF document(s).`);
      } catch (operationError) {
        const message =
          operationError instanceof Error
            ? operationError.message
            : "StarPDF could not merge the selected documents.";
        setError(message);
        toast.error(message);
      } finally {
        if (mergeInputRef.current) mergeInputRef.current.value = "";
        setIsPageProcessing(false);
      }
    },
    [cleanupProxy, filename, isModified, isPageProcessing, loadDocument, sourceBytes],
  );

  const handleTogglePageSelection = useCallback((pageNumber: number) => {
    setSelectedPages((previous) => {
      const next = new Set(previous);
      if (next.has(pageNumber)) next.delete(pageNumber);
      else next.add(pageNumber);
      return next;
    });
  }, []);

  const handleOpenNewFile = useCallback(() => {
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
  }, [cleanupProxy]);

  // Dropzone screen when no document is active
  if (!sourceBytes || !inspectionResult) {
    return (
      <div className="py-6 sm:py-12">
        <PdfDropzone
          onFileSelect={loadDocument}
          isLoading={isLoading}
          loadingMessage={loadingMessage}
          error={error}
        />
      </div>
    );
  }

  return (
    <div
      className="flex flex-col h-[820px] rounded-3xl border border-slate-200 bg-slate-100 overflow-hidden shadow-xs"
      data-testid="smartpdf-editor-workspace"
    >
      {securityInfo && securityInfo.signature_state !== "UNSIGNED" ? (
        <div
          className="border-b border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-950"
          data-testid="starpdf-signed-document-warning"
          role="status"
        >
          This PDF contains a digital signature. StarPDF preserves the original signed bytes when
          appending an update, but it does not verify cryptographic signature validity. A saved edit
          is a post-signature revision.
        </div>
      ) : null}
      {/* Top Application Toolbar */}
      <PdfToolbar
        filename={filename}
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        scale={scale}
        isExporting={isExporting}
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
        onOpenNewFile={handleOpenNewFile}
      />

      <PdfPageOperations
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        selectedCount={selectedPages.size}
        isProcessing={isPageProcessing}
        onDelete={() =>
          void applyPageOperation(
            { type: "deletePage", pageIndex: currentPage - 1 },
            Math.min(currentPage, inspectionResult.metadata.pageCount - 1),
            "Page deleted.",
          )
        }
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
              width: inspectionResult.pages[currentPage - 1]?.width || 612,
              height: inspectionResult.pages[currentPage - 1]?.height || 792,
              rotation: 0,
            },
            currentPage + 1,
            "Blank page inserted.",
          )
        }
        onExtract={() => void handleExtractPages()}
        onMerge={() => mergeInputRef.current?.click()}
      />
      <input
        ref={mergeInputRef}
        type="file"
        accept="application/pdf,.pdf"
        multiple
        className="hidden"
        aria-label="Add PDF documents"
        onChange={(event) => void handleMergeFiles(event.target.files)}
      />

      {/* Main Workspace: Thumbnails + Viewport Canvas + Form Inspector */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Thumbnail Rail */}
        <PdfThumbnailRail
          pdfDocument={pdfProxy}
          pageCount={inspectionResult.metadata.pageCount}
          currentPage={currentPage}
          onPageSelect={setCurrentPage}
          selectedPages={selectedPages}
          onToggleSelection={handleTogglePageSelection}
          className="hidden md:block"
        />

        {/* Center Viewport Canvas Surface */}
        <main
          ref={viewportContainerRef}
          className="flex-1 bg-slate-100/90 overflow-auto p-6 sm:p-8 flex items-center justify-center min-w-0"
          aria-label="PDF Document Page Viewport"
        >
          <div className="my-auto transition-transform duration-75">
            <PdfPageCanvas
              pdfDocument={pdfProxy}
              pageNumber={currentPage}
              scale={scale}
              rotation={inspectionResult.pages[currentPage - 1]?.rotation || 0}
            />
          </div>
        </main>

        {/* Right Form Fields, Text & Image Inspector */}
        <PdfFormInspector
          fields={inspectionResult.fields}
          fieldValues={fieldValues}
          onFieldValueChange={handleFieldValueChange}
          onResetForm={handleResetForm}
          isModified={isModified}
          textSpans={pageTextSpans}
          onReplaceText={handleReplaceExistingText}
          images={pageImages}
          onReplaceImage={handleReplaceImage}
          onAddImage={handleAddImage}
          onRemoveImage={handleRemoveImage}
          graphics={pageGraphics}
          onUpdateGraphic={handleUpdateGraphic}
          onAddRectangle={handleAddRectangle}
          onAddLine={handleAddLine}
          onDeleteGraphic={handleDeleteGraphic}
          className="hidden lg:flex"
        />
      </div>

      {/* Document Properties Modal */}
      <PdfDocumentInfo
        metadata={inspectionResult.metadata}
        isOpen={showInfoModal}
        onClose={() => setShowInfoModal(false)}
      />
    </div>
  );
}
