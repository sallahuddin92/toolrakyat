"use client";

import { useState, useCallback, useRef, useEffect, useMemo } from "react";

import { getPdfjsLib, type PDFDocumentProxy } from "@/lib/pdf/pdfjs-init";
import toast from "react-hot-toast";

import { inspectPdfDocument } from "@/lib/pdf/pdf-engine";
import {
  type DocumentInspectionResult,
} from "@/lib/pdf/pdf-types";
import { StarPdfClient, type StarPdfDocumentHandle } from "@/lib/pdf/starpdf-client";
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

import {
  type SmartPdfSelection,
  resolveSelectionAfterMutation,
} from "@/lib/pdf/selection";
import {
  type SmartPdfCommand,
  type SmartPdfCommandContext,
  type CommandExecutionState,
  type SmartPdfHistoryState,
  createInitialHistoryState,
  pushHistorySnapshot,
  canUndo,
  canRedo,
  undoHistory,
  redoHistory,
  ReplaceTextCommand,
  DeleteTextCommand,
  MoveTextCommand,
  ReplaceImageCommand,
  UpdateImageCommand,
  RemoveImageCommand,
  AddImageCommand,
  UpdateVectorCommand,
  DeleteVectorCommand,
  AddFreeTextCommand,
  AddCheckMarkCommand,
  AddCrossMarkCommand,
  AddInkAnnotationCommand,
  DeleteAnnotationCommand,
  UpdateAnnotationRectCommand,
  MovePageCommand,
  DuplicatePageCommand,
  DeletePageCommand,
  InsertBlankPageCommand,
  ExtractPagesCommand,
  MergeDocumentsCommand,
  ExportDocumentCommand,
} from "@/lib/pdf/commands";

import {
  type FlatFormCandidate,
  detectFlatFormCandidates,
  deduplicateCandidates,
} from "@/lib/pdf/detection";

import { Button } from "@/components/ui/button";
import { PdfDropzone } from "./PdfDropzone";

import { PdfToolbar, type EditorMode, type FillAndSignTool } from "./PdfToolbar";
import { PdfThumbnailRail } from "./PdfThumbnailRail";
import { PdfPageCanvas } from "./PdfPageCanvas";
import { PdfDocumentInfo } from "./PdfDocumentInfo";
import { PdfPageOperations } from "./PdfPageOperations";
import { PdfContextualToolbar } from "./PdfContextualToolbar";
import { PdfConfirmDialog } from "./PdfConfirmDialog";

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

  // Application Tool Mode System (Phase 3A / 3A.1)
  const [editorMode, setEditorMode] = useState<EditorMode>("SELECT");
  const [fillAndSignTool, setFillAndSignTool] = useState<FillAndSignTool>("text");
  const [showFlatFormBanner, setShowFlatFormBanner] = useState<boolean>(true);
  const [formAssistEnabled, setFormAssistEnabled] = useState<boolean>(true);
  const [rasterCandidates, setRasterCandidates] = useState<Record<number, FlatFormCandidate[]>>({});
  const [signatureFile, setSignatureFile] = useState<File | null>(null);
  const signatureInputRef = useRef<HTMLInputElement | null>(null);


  // Unified selection state on canvas
  const [selectedItem, setSelectedItem] = useState<SmartPdfSelection>(null);

  // Command Execution State (IDLE vs RUNNING)
  const [commandState, setCommandState] = useState<CommandExecutionState>({ status: "IDLE" });

  // Centralized Bounded 25-Snapshot Transaction History
  const [historyState, setHistoryState] = useState<SmartPdfHistoryState>(() =>
    createInitialHistoryState(new Uint8Array(0)),
  );
  const historyStateRef = useRef<SmartPdfHistoryState>(historyState);
  useEffect(() => {
    historyStateRef.current = historyState;
  }, [historyState]);


  // Unsaved changes confirmation dialog
  const [showConfirmOpenModal, setShowConfirmOpenModal] = useState<boolean>(false);

  // Layout Panel Visibility (Desktop Workspace)
  const [isThumbnailsOpen, setIsThumbnailsOpen] = useState<boolean>(true);

  // StarPDF search state
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [searchResults, setSearchResults] = useState<StarPdfSearchResult[]>([]);
  const [activeSearchIndex, setActiveSearchIndex] = useState<number>(0);

  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [loadingMessage, setLoadingMessage] = useState<string>("Loading document...");
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

  const loadDocument = useCallback(
    async (
      bytes: Uint8Array,
      docFilename: string,
      docSize: number,
      initialPage = 1,
      isHistoryRestore = false,
    ) => {
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
          const initialHistory = createInitialHistoryState(bytes, "Initial document");
          setHistoryState(initialHistory);
          if (inspected.fields.length > 0) {
            toast.success(
              `Loaded "${docFilename}" with ${inspected.fields.length} interactive form field(s).`,
            );
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

  /**
   * Centralized Command Execution Lifecycle
   * Coordinates validation, busy state, execution, atomic document refresh,
   * bounded history, dirty state, selection resolution, error translation, and user feedback.
   */
  const isBusyRef = useRef(false);
  useEffect(() => {
    isBusyRef.current = commandState.status === "RUNNING";
  }, [commandState.status]);

  const executeCommand = useCallback(
    async (command: SmartPdfCommand): Promise<void> => {
      const isLightweight = command.id === "form.set_value" || command.id === "annotation.update";


      if (!isLightweight) {
        // If currently running a previous operation, wait up to 2.5s for it to settle
        let waitCount = 0;
        while (isBusyRef.current && waitCount < 50) {
          await new Promise((resolve) => setTimeout(resolve, 50));
          waitCount++;
        }

        if (isBusyRef.current) {
          console.warn(`Command "${command.label}" rejected: executor is currently busy.`);
          return;
        }

        setCommandState({
          status: "RUNNING",
          commandId: command.id,
          label: command.label,
        });
      }

      const context: SmartPdfCommandContext = {
        sourceBytes,
        filename,
        currentPage,
        pageCount: inspectionResult?.metadata.pageCount || 1,
        selection: selectedItem,
        starPdfDoc,
        fieldValues,
        annotationValues,
        inspectionResult,
      };

      try {
        const result = await command.execute(context);

        // 1. Handle browser file download if returned
        if (result.download) {
          const blob = new Blob([result.download.bytes as unknown as BlobPart], {
            type: "application/pdf",
          });
          const url = URL.createObjectURL(blob);
          const a = document.createElement("a");
          a.href = url;
          a.download = result.download.filename;
          document.body.appendChild(a);
          a.click();
          document.body.removeChild(a);
          URL.revokeObjectURL(url);
        }

        // 2. Handle document byte mutations
        if (result.bytes) {
          const targetPage = result.nextPage !== undefined ? result.nextPage : currentPage;
          await loadDocument(
            result.bytes,
            filename,
            result.bytes.byteLength,
            targetPage,
            true, // isHistoryRestore flag prevents resetting history
          );

          if (command.isMutating) {
            setHistoryState((prev) => pushHistorySnapshot(prev, result.bytes!, command.label));
            setIsModified(true);
          }
        } else {
          // Non-byte mutating state updates (e.g. form fields, annotations)
          if (result.fieldValues) {
            const updatedFields = result.fieldValues;
            setFieldValues((prev) => ({ ...prev, ...updatedFields }));
            if (command.isMutating) {
              if (sourceBytes) {
                setHistoryState((prev) =>
                  pushHistorySnapshot(prev, sourceBytes, command.label),
                );
              }
              setIsModified(true);
            }
          }
          if (result.annotationValues) {
            const updatedAnnots = result.annotationValues;
            setAnnotationValues((prev) => ({ ...prev, ...updatedAnnots }));
            if (command.isMutating) {
              if (sourceBytes) {
                setHistoryState((prev) =>
                  pushHistorySnapshot(prev, sourceBytes, command.label),
                );
              }
              setIsModified(true);
            }
          }
          if (result.nextPage !== undefined) {
            setCurrentPage(result.nextPage);
          }
        }

        // 3. Selection resolution / cleanup
        if (result.clearSelection) {
          setSelectedItem(null);
        } else if (result.nextSelection !== undefined) {
          setSelectedItem(result.nextSelection);
        } else if (command.isMutating) {
          const targetPageIndex = (result.nextPage || currentPage) - 1;
          setSelectedItem((prev) =>
            resolveSelectionAfterMutation(
              prev,
              targetPageIndex,
              pageTextSpans,
              pageImages,
              pageGraphics,
              inspectionResult?.fields || [],
              inspectionResult?.annotations || [],
            ),
          );
        }

        // 4. Concise user feedback
        if (result.message && !isLightweight) {
          toast.success(result.message);
        }
      } catch (err: unknown) {
        const friendly = formatPdfErrorMessage(err);
        toast.error(friendly.userMessage);
      } finally {
        if (!isLightweight) {
          setCommandState({ status: "IDLE" });
        }
      }
    },

    [
      annotationValues,
      currentPage,
      fieldValues,
      filename,
      inspectionResult,
      loadDocument,
      pageGraphics,
      pageImages,
      pageTextSpans,
      selectedItem,
      sourceBytes,
      starPdfDoc,
    ],

  );

  const handleUndo = useCallback(async () => {
    const undone = undoHistory(historyStateRef.current);
    if (!undone) return;

    setHistoryState(undone.nextState);
    await loadDocument(
      undone.entry.bytes,
      filename,
      undone.entry.bytes.byteLength,
      currentPage,
      true,
    );
    toast.success(`Undo: ${historyStateRef.current.snapshots[historyStateRef.current.currentIndex].description}`);
  }, [currentPage, filename, loadDocument]);

  const handleRedo = useCallback(async () => {
    const redone = redoHistory(historyStateRef.current);
    if (!redone) return;

    setHistoryState(redone.nextState);
    await loadDocument(
      redone.entry.bytes,
      filename,
      redone.entry.bytes.byteLength,
      currentPage,
      true,
    );
    toast.success(`Redo: ${redone.entry.description}`);
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

  const handlePageNavigation = useCallback((newPage: number) => {
    setCurrentPage((prev) => {
      if (prev !== newPage) {
        setSelectedItem(null);
      }
      return newPage;
    });
  }, []);

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
    setAnnotationValues({});
    setIsModified(false);
    setSelectedPages(new Set([1]));
    setError(null);
    setSecurityInfo(null);
    setCurrentPage(1);
    setScale(1.0);
    setSearchQuery("");
    setSearchResults([]);
    setHistoryState(createInitialHistoryState(new Uint8Array(0)));
    setSelectedItem(null);
    setEditorMode("SELECT");
    setFillAndSignTool("text");
    setSignatureFile(null);
    setShowFlatFormBanner(true);
  }, [cleanupProxy]);

  const handleOpenNewFileClick = useCallback(() => {
    if (isModified) {
      setShowConfirmOpenModal(true);
    } else {
      performOpenNewFile();
    }
  }, [isModified, performOpenNewFile]);

  // Creation & Placement Handlers
  const handlePlaceFreeText = useCallback(
    (pageIndex: number, x: number, y: number, text: string) => {
      void executeCommand(new AddFreeTextCommand(pageIndex, x, y, text, 12, [0, 0, 0]));
    },
    [executeCommand],
  );

  const handlePlaceCheck = useCallback(
    (pageIndex: number, x: number, y: number) => {
      void executeCommand(new AddCheckMarkCommand(pageIndex, x, y, 16));
    },
    [executeCommand],
  );

  const handlePlaceCross = useCallback(
    (pageIndex: number, x: number, y: number) => {
      void executeCommand(new AddCrossMarkCommand(pageIndex, x, y, 16));
    },
    [executeCommand],
  );

  const handlePlaceSignature = useCallback(
    (pageIndex: number, x: number, y: number) => {
      if (signatureFile) {
        void executeCommand(new AddImageCommand(pageIndex, x, y, 120, 40, signatureFile));
      } else {
        signatureInputRef.current?.click();
      }
    },
    [signatureFile, executeCommand],
  );

  const handlePlaceDrawing = useCallback(
    (
      pageIndex: number,
      inkList: [number, number][][],
      rect: [number, number, number, number],
    ) => {
      void executeCommand(new AddInkAnnotationCommand(pageIndex, inkList, rect, [0, 0, 0.8], 2));
    },
    [executeCommand],
  );

  const handleTransformImage = useCallback(
    async (pageIndex: number, imageId: string, pdfRect: import("@/lib/pdf/selection").PdfRect) => {
      await executeCommand(
        new UpdateImageCommand(
          imageId,
          pdfRect.x,
          pdfRect.y,
          pdfRect.width,
          pdfRect.height,
          pageIndex,
        ),
      );
    },
    [executeCommand],
  );

  const handleTransformVector = useCallback(
    async (
      pageIndex: number,
      graphicId: string,
      pdfRect: import("@/lib/pdf/selection").PdfRect,
      linePoints?: { x1: number; y1: number; x2: number; y2: number },
    ) => {
      if (linePoints) {
        await executeCommand(
          new UpdateVectorCommand({
            page_index: pageIndex,
            graphic_id: graphicId,
            line_x1: linePoints.x1,
            line_y1: linePoints.y1,
            line_x2: linePoints.x2,
            line_y2: linePoints.y2,
          }),
        );
      } else {
        await executeCommand(
          new UpdateVectorCommand({
            page_index: pageIndex,
            graphic_id: graphicId,
            rect_x: pdfRect.x,
            rect_y: pdfRect.y,
            rect_w: pdfRect.width,
            rect_h: pdfRect.height,
          }),
        );
      }
    },
    [executeCommand],
  );

  const handleMoveText = useCallback(
    async (pageIndex: number, spanId: string | string[], dx: number, dy: number) => {
      try {
        await executeCommand(new MoveTextCommand(spanId, dx, dy, pageIndex + 1));
      } catch (err: unknown) {
        console.error("Text move failed:", err);
        const errMsg = err instanceof Error ? err.message : String(err);
        toast.error(
          errMsg.includes("TEXT_MOVE")
            ? `Cannot move text: ${errMsg}`
            : "Cannot move this text safely. Surrounding layout depends on its position.",
        );
      }
    },
    [executeCommand],
  );

  const handleTransformAnnotation = useCallback(
    async (pageIndex: number, annotId: string, pdfRect: import("@/lib/pdf/selection").PdfRect) => {
      await executeCommand(
        new UpdateAnnotationRectCommand(
          annotId,
          [pdfRect.x, pdfRect.y, pdfRect.x + pdfRect.width, pdfRect.y + pdfRect.height],
          pageIndex + 1,
        ),
      );
    },
    [executeCommand],
  );

  const handleDeleteAnnotation = useCallback(

    async (annotId: string) => {
      await executeCommand(new DeleteAnnotationCommand(currentPage - 1, annotId));
    },
    [currentPage, executeCommand],
  );


  const handleFormFieldChange = useCallback(
    (fieldName: string, value: string | boolean | string[]) => {
      setFieldValues((prev) => ({ ...prev, [fieldName]: value }));
      setIsModified(true);
      if (sourceBytes) {
        setHistoryState((prev) =>
          pushHistorySnapshot(prev, sourceBytes, `Update field "${fieldName}"`),
        );
      }
    },
    [sourceBytes],
  );

  const handleAnnotationChange = useCallback(
    (annotId: string, value: string) => {
      setAnnotationValues((prev) => ({ ...prev, [annotId]: value }));
      setIsModified(true);
      if (sourceBytes) {
        setHistoryState((prev) =>
          pushHistorySnapshot(prev, sourceBytes, "Update annotation"),
        );
      }
    },
    [sourceBytes],
  );

  // Global Keyboard Shortcuts (Escape to cancel tool mode or selection, Delete/Backspace for selected objects)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (editorMode === "FILL_AND_SIGN") {
          setEditorMode("SELECT");
        } else if (selectedItem) {
          setSelectedItem(null);
        }
        return;
      }

      if (e.key === "Delete" || e.key === "Backspace") {
        const target = e.target;
        if (
          target instanceof HTMLInputElement ||
          target instanceof HTMLTextAreaElement ||
          (target instanceof HTMLElement && target.isContentEditable)
        ) {
          return;
        }

        if (selectedItem && !isBusyRef.current) {
          e.preventDefault();
          const item = selectedItem;
          if (item.type === "annotation") {
            void handleDeleteAnnotation(item.id);
          } else if (item.type === "image") {
            setSelectedItem(null);
            void executeCommand(new RemoveImageCommand(item.id));
          } else if (item.type === "vector") {
            setSelectedItem(null);
            void executeCommand(new DeleteVectorCommand(item.id));
          } else if (item.type === "text") {
            const isSingleEditable = item.group
              ? item.group.editability === "EDITABLE_ATOMIC"
              : item.data.is_editable;
            if (isSingleEditable) {
              const targetSpanIds =
                item.group && item.group.sourceSpans.length > 0
                  ? item.group.sourceSpans.map((s) => s.span_id)
                  : item.data.span_id;
              setSelectedItem(null);
              void executeCommand(new DeleteTextCommand(targetSpanIds));
            } else {
              toast.error("This text can't be safely removed in place.");
            }
          } else if (item.type === "form") {

            handleFormFieldChange(item.id, "");
          }
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [editorMode, selectedItem, handleDeleteAnnotation, executeCommand, handleFormFieldChange]);


  // Form Affordance Detection (Form Assist)
  const currentCandidates = useMemo(() => {
    if (!inspectionResult) return [];
    const pageIndex = currentPage - 1;
    const pageWidth = inspectionResult.pages[pageIndex]?.width || 612;
    const pageHeight = inspectionResult.pages[pageIndex]?.height || 792;

    const baseCandidates = detectFlatFormCandidates({
      pageIndex,
      pageWidth,
      pageHeight,
      acroFields: inspectionResult.fields,
      vectorGraphics: pageGraphics,
    });

    const pageRaster = rasterCandidates[pageIndex] || [];
    return deduplicateCandidates([...baseCandidates, ...pageRaster]);
  }, [currentPage, inspectionResult, pageGraphics, rasterCandidates]);

  const handleCanvasRendered = useCallback(
    (canvas: HTMLCanvasElement) => {
      if (!inspectionResult) return;
      const pageIndex = currentPage - 1;
      const pageWidth = inspectionResult.pages[pageIndex]?.width || 612;
      const pageHeight = inspectionResult.pages[pageIndex]?.height || 792;

      try {
        const ctx = canvas.getContext("2d", { willReadFrequently: true });
        if (ctx) {
          const imgData = ctx.getImageData(0, 0, canvas.width, canvas.height);
          const fullCandidates = detectFlatFormCandidates({
            pageIndex,
            pageWidth,
            pageHeight,
            acroFields: inspectionResult.fields,
            vectorGraphics: pageGraphics,
            imageData: imgData,
          });
          setRasterCandidates((prev) => ({
            ...prev,
            [pageIndex]: fullCandidates,
          }));
        }
      } catch {
        // Ignore canvas read errors if tainted
      }
    },
    [currentPage, inspectionResult, pageGraphics],
  );

  const isFlatForm = Boolean(


    inspectionResult &&
      inspectionResult.fields.length === 0 &&
      pageTextSpans.filter((s) => s.text.trim().length > 0).length === 0 &&
      pageImages.length >= 1,
  );


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

  const isBusy = commandState.status === "RUNNING";

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

      {/* Flat Form Suggestion Banner */}
      {showFlatFormBanner && isFlatForm && (
        <div
          role="status"
          className="bg-amber-50 border-b border-amber-200 px-4 py-2 flex items-center justify-between text-xs text-amber-900 shrink-0 select-none animate-in fade-in duration-100"
          data-testid="flat-form-suggestion-banner"
        >
          <div className="flex items-center gap-2 min-w-0">
            <span className="font-semibold bg-amber-200 text-amber-900 px-1.5 py-0.5 rounded text-[11px]">Notice</span>
            <span className="truncate">This appears to be a flat form. Use Fill & Sign to type anywhere on the page.</span>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            <Button
              type="button"
              size="sm"
              variant="outline"
              onClick={() => {
                setEditorMode("FILL_AND_SIGN");
                setShowFlatFormBanner(false);
              }}
              className="h-7 text-xs bg-white border-amber-300 text-amber-900 hover:bg-amber-100"
              data-testid="flat-form-fill-sign-btn"
            >
              Use Fill & Sign
            </Button>
            <button
              type="button"
              onClick={() => setShowFlatFormBanner(false)}
              className="text-amber-600 hover:text-amber-900 p-1 text-xs"
              aria-label="Dismiss flat form notice"
            >
              ✕
            </button>
          </div>
        </div>
      )}

      {/* Top Application Toolbar */}
      <PdfToolbar
        filename={filename}
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        scale={scale}
        isExporting={isBusy && commandState.commandId === "document.export"}
        isModified={isModified}
        canUndo={canUndo(historyState)}
        canRedo={canRedo(historyState)}
        onUndo={() => void handleUndo()}
        onRedo={() => void handleRedo()}
        searchQuery={searchQuery}
        searchResultCount={searchResults.length}
        activeSearchIndex={activeSearchIndex}
        onSearchChange={(q) => void handleSearchChange(q)}
        onNextSearchResult={handleNextSearchResult}
        onPrevSearchResult={handlePrevSearchResult}
        onPageChange={handlePageNavigation}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onFitWidth={handleFitWidth}
        onFitPage={handleFitPage}
        onExport={async (mode) => {
          await executeCommand(new ExportDocumentCommand(mode));
        }}
        onShowInfo={() => setShowInfoModal(true)}
        onOpenNewFile={handleOpenNewFileClick}
        onMergeClick={() => mergeInputRef.current?.click()}
        isThumbnailsOpen={isThumbnailsOpen}
        onToggleThumbnails={() => setIsThumbnailsOpen(!isThumbnailsOpen)}
        mode={editorMode}
        onModeChange={setEditorMode}
        fillAndSignTool={fillAndSignTool}
        onFillAndSignToolChange={(tool) => {
          setFillAndSignTool(tool);
          if (tool === "signature" && !signatureFile) {
            signatureInputRef.current?.click();
          }
        }}
        formAssistEnabled={formAssistEnabled}
        onToggleFormAssist={() => setFormAssistEnabled((prev) => !prev)}
      />


      {/* Page Operations Rail / Actions */}
      <PdfPageOperations
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        selectedCount={selectedPages.size}
        isProcessing={isBusy}
        onDelete={() => {
          if (inspectionResult.metadata.pageCount <= 1) {
            toast.error("Cannot delete the only page in a document.");
            return;
          }
          void executeCommand(new DeletePageCommand(currentPage - 1));
        }}
        onMoveLeft={() => {
          if (currentPage <= 1) return;
          void executeCommand(new MovePageCommand(currentPage - 1, currentPage - 2));
        }}
        onMoveRight={() => {
          if (currentPage >= inspectionResult.metadata.pageCount) return;
          void executeCommand(new MovePageCommand(currentPage - 1, currentPage));
        }}
        onDuplicate={() => {
          void executeCommand(new DuplicatePageCommand(currentPage - 1));
        }}
        onInsertBlank={() => {
          const currentPageInfo = inspectionResult.pages[currentPage - 1] || {
            width: 612,
            height: 792,
            rotation: 0,
          };
          void executeCommand(
            new InsertBlankPageCommand(
              currentPage, // insert after current page
              currentPageInfo.width,
              currentPageInfo.height,
              currentPageInfo.rotation as 0 | 90 | 180 | 270,
            ),
          );
        }}
        onExtract={() => {
          void executeCommand(new ExtractPagesCommand(Array.from(selectedPages)));
        }}
        onMerge={() => mergeInputRef.current?.click()}
      />

      {/* Hidden File Input for Adding/Merging PDFs */}
      <input
        ref={mergeInputRef}
        type="file"
        accept="application/pdf"
        multiple
        className="hidden"
        aria-label="Add PDF documents"
        onChange={async (e) => {
          const files = e.target.files;
          if (files && files.length > 0) {
            const additions = await Promise.all(
              Array.from(files).map(async (file) => new Uint8Array(await file.arrayBuffer())),
            );
            void executeCommand(new MergeDocumentsCommand(additions));
          }
          if (mergeInputRef.current) mergeInputRef.current.value = "";
        }}
      />

      {/* Hidden File Input for Signature Image */}
      <input
        ref={signatureInputRef}
        type="file"
        accept="image/png,image/jpeg,image/jpg"
        className="hidden"
        aria-label="Upload signature image"
        onChange={(e) => {
          const files = e.target.files;
          if (files && files.length > 0) {
            setSignatureFile(files[0]);
            toast.success("Signature image selected. Click anywhere on the document to place it.");
          }
        }}
      />

      {/* Main Workspace Body */}
      <div className="flex-1 flex overflow-hidden relative">
        {/* Left Collapsible Thumbnail Rail */}
        {isThumbnailsOpen && (
          <PdfThumbnailRail
            pdfDocument={pdfProxy}
            pageCount={inspectionResult.metadata.pageCount}
            currentPage={currentPage}
            selectedPages={selectedPages}
            onPageSelect={handlePageNavigation}
            onToggleSelection={handleTogglePageSelection}
          />
        )}

        {/* Center Viewport Canvas Surface */}
        <main
          ref={viewportContainerRef}
          className="flex-1 bg-slate-100/90 overflow-auto p-6 sm:p-8 flex items-center justify-center min-w-0 relative"
          aria-label="PDF Document Page Viewport"
          onClick={(e) => {
            if (e.target === e.currentTarget && editorMode === "SELECT") {
              setSelectedItem(null);
            }
          }}
        >

          {/* Floating Contextual Object Action Bar */}
          {selectedItem && (
            <PdfContextualToolbar
              selection={selectedItem}
              containerRef={viewportContainerRef}
              onDeselect={() => setSelectedItem(null)}
              onReplaceText={async (spanId, newText) => {
                await executeCommand(new ReplaceTextCommand(spanId, newText));
              }}
              onDeleteText={async (spanId) => {
                setSelectedItem(null);
                await executeCommand(new DeleteTextCommand(spanId));
              }}
              onReplaceImage={async (imageId, file) => {
                await executeCommand(new ReplaceImageCommand(imageId, file));
              }}
              onRemoveImage={async (imageId) => {
                await executeCommand(new RemoveImageCommand(imageId));
              }}
              onUpdateGraphic={async (input: StarPdfUpdateVectorGraphicInput) => {
                await executeCommand(new UpdateVectorCommand(input));
              }}
              onDeleteGraphic={async (graphicId) => {
                await executeCommand(new DeleteVectorCommand(graphicId));
              }}
              onFormFieldChange={handleFormFieldChange}
              formFieldValue={
                selectedItem.type === "form" ? fieldValues[selectedItem.id] : undefined
              }
              onAnnotationChange={handleAnnotationChange}
              annotationValue={
                selectedItem.type === "annotation" ? annotationValues[selectedItem.id] : undefined
              }
              onDeleteAnnotation={handleDeleteAnnotation}
              onAddTextInstead={() => {
                setEditorMode("FILL_AND_SIGN");
                setFillAndSignTool("text");
              }}
            />
          )}

          <div className="my-auto transition-transform duration-75">
            <PdfPageCanvas
              pdfDocument={pdfProxy}
              pageNumber={currentPage}
              scale={scale}
              rotation={inspectionResult.pages[currentPage - 1]?.rotation || 0}
              pageWidth={inspectionResult.pages[currentPage - 1]?.width || 612}
              pageHeight={inspectionResult.pages[currentPage - 1]?.height || 792}
              textSpans={pageTextSpans}
              images={pageImages}
              graphics={pageGraphics}
              fields={inspectionResult.fields}
              annotations={inspectionResult.annotations}
              candidates={currentCandidates}
              formAssistEnabled={formAssistEnabled}
              selectedItem={selectedItem}
              onSelectItem={setSelectedItem}
              onCanvasRendered={handleCanvasRendered}
              mode={editorMode}
              fillAndSignTool={fillAndSignTool}
              onPlaceFreeText={handlePlaceFreeText}
              onPlaceCheck={handlePlaceCheck}
              onPlaceCross={handlePlaceCross}
              onPlaceSignature={handlePlaceSignature}
              onPlaceDrawing={handlePlaceDrawing}
              onTransformImage={handleTransformImage}
              onTransformVector={handleTransformVector}
              onMoveText={handleMoveText}
              onTransformAnnotation={handleTransformAnnotation}
            />
          </div>

        </main>
      </div>


      {/* Bottom Application Status Bar */}
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
          {isBusy && (
            <>
              <span className="text-slate-300">•</span>
              <span className="text-amber-600 font-medium animate-pulse">{commandState.label}…</span>
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

      {/* Confirm Open New File Modal */}
      <PdfConfirmDialog
        isOpen={showConfirmOpenModal}
        title="Unsaved Changes"
        description="You have unsaved changes in the current document. Opening a new file will discard these changes. Are you sure you want to proceed?"
        confirmLabel="Discard & Open New"
        cancelLabel="Cancel"
        onConfirm={performOpenNewFile}
        onCancel={() => setShowConfirmOpenModal(false)}
      />
    </div>
  );
}
