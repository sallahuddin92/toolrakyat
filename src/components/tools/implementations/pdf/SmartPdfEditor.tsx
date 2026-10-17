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
import type { EvidenceLevel } from "@/lib/pdf/execution/contracts";
import { AlertTriangle, ShieldCheck, FolderOpen } from "lucide-react";

import {
  type SmartPdfSelection,
  resolveSelectionAfterMutation,
  selectionNodeId,
  searchPdfPalette,
  type PdfPaletteCommandId,
} from "@/lib/pdf/selection";
import {
  annotationWithFreeTextState,
  cloneFreeTextState,
  freeTextSessionNeedsSync,
  freeTextStateFromAnnotation,
  freeTextStateToUpdatePatch,
  freeTextStatesEqual,
  type FreeTextEditSession,
  type FreeTextEditState,
} from "@/lib/pdf/freetext-edit-session";
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
  ApplyTextStyleCommand,
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
  UpdateAnnotationCommand,
  UpdateAnnotationPropertiesCommand,
  UpdateAnnotationRectCommand,
  MovePageCommand,
  ReorderPagesCommand,
  DuplicatePageCommand,
  DuplicatePagesBatchCommand,
  DeletePageCommand,
  DeletePagesBatchCommand,
  InsertBlankPageCommand,
  ExtractPagesCommand,
  InsertImportedPagesCommand,
  SplitDocumentCommand,
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
import { SmartPdfImportModal } from "./SmartPdfImportModal";
import { SmartPdfSplitModal } from "./SmartPdfSplitModal";
import { SmartPdfHome } from "@/components/smartpdf/home/SmartPdfHome";
import { SmartPdfAllTools } from "@/components/smartpdf/tools/SmartPdfAllTools";
import { SmartPdfNavbar } from "@/components/smartpdf/layout/SmartPdfNavbar";
import { addRecentDocument } from "@/components/smartpdf/recent/recentFilesStore";
import type { SmartPdfSurface, PdfToolDefinition } from "@/components/smartpdf/types";

export interface SmartPdfEditorProps {
  initialFile?: {
    filename: string;
    bytes: Uint8Array;
  };
  initialMode?: EditorMode;
  onHomeClick?: () => void;
  onAllToolsClick?: () => void;
  onDocumentLoaded?: (doc: { filename: string; bytes: Uint8Array; pageCount: number }) => void;
}

export function SmartPdfEditor({
  initialFile,
  initialMode,
  onHomeClick: externalHomeClick,
  onAllToolsClick: externalAllToolsClick,
  onDocumentLoaded,
}: SmartPdfEditorProps = {}) {
  const [sourceBytes, setSourceBytes] = useState<Uint8Array | null>(null);
  const [filename, setFilename] = useState<string>("");
  const [inspectionResult, setInspectionResult] = useState<DocumentInspectionResult | null>(null);
  const [pdfProxy, setPdfProxy] = useState<PDFDocumentProxy | null>(null);
  const [starPdfDoc, setStarPdfDoc] = useState<StarPdfDocumentHandle | null>(null);
  const [securityInfo, setSecurityInfo] = useState<StarPdfSecurityInfo | null>(null);
  const [nativeEngineState, setNativeEngineState] = useState<
    "READY" | "RECOVERED_MALFORMED_PREV" | "READ_ONLY"
  >("READY");
  const [evidenceLevel, setEvidenceLevel] = useState<EvidenceLevel>("UNTRUSTED");

  const [currentPage, setCurrentPage] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [viewRotation, setViewRotation] = useState(0);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState("");
  const [paletteIndex, setPaletteIndex] = useState(0);
  const [fieldValues, setFieldValues] = useState<Record<string, string | boolean | string[]>>({});
  const [annotationValues, setAnnotationValues] = useState<Record<string, string>>({});
  const [pageTextSpans, setPageTextSpans] = useState<StarPdfTextSpan[]>([]);
  const [pageImages, setPageImages] = useState<StarPdfImageInfo[]>([]);
  const [pageGraphics, setPageGraphics] = useState<StarPdfVectorGraphicInfo[]>([]);
  const [isModified, setIsModified] = useState<boolean>(false);
  const [selectedPages, setSelectedPages] = useState<Set<number>>(() => new Set([1]));

  // Top-level 3-Surface Navigation: "home" | "tools" | "workspace"
  const [surface, setSurface] = useState<SmartPdfSurface>("home");

  // Application Tool Mode System (Phase 3A / 3A.1)
  const [editorMode, setEditorMode] = useState<EditorMode>(initialMode || "SELECT");
  const [fillAndSignTool, setFillAndSignTool] = useState<FillAndSignTool>("text");
  const [showFlatFormBanner, setShowFlatFormBanner] = useState<boolean>(true);
  const [formAssistEnabled, setFormAssistEnabled] = useState<boolean>(true);
  const [rasterCandidates, setRasterCandidates] = useState<Record<number, FlatFormCandidate[]>>({});
  const [signatureFile, setSignatureFile] = useState<File | null>(null);
  const signatureInputRef = useRef<HTMLInputElement | null>(null);
  const imageInputRef = useRef<HTMLInputElement | null>(null);


  // Unified selection state on canvas
  const [selectedItem, setSelectedItem] = useState<SmartPdfSelection>(null);
  const [freeTextSession, setFreeTextSession] = useState<FreeTextEditSession | null>(null);
  const freeTextSessionRef = useRef<FreeTextEditSession | null>(null);
  const freeTextCommitRef = useRef<Promise<void> | null>(null);
  const sessionNumberRef = useRef(0);
  const sourceBytesRef = useRef<Uint8Array | null>(null);
  const inspectionResultRef = useRef<DocumentInspectionResult | null>(null);
  const annotationValuesRef = useRef<Record<string, string>>({});
  const fieldValuesRef = useRef<Record<string, string | boolean | string[]>>({});

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

  // Multi-document / page organizer modals (Phase 6)
  const [showImportModal, setShowImportModal] = useState<boolean>(false);
  const [showSplitModal, setShowSplitModal] = useState<boolean>(false);

  // Layout Panel Visibility (Desktop Workspace)
  const [isThumbnailsOpen, setIsThumbnailsOpen] = useState<boolean>(true);

  // StarPDF search state (Phase 7)
  const [isSearchOpen, setIsSearchOpen] = useState<boolean>(false);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [searchResults, setSearchResults] = useState<StarPdfSearchResult[]>([]);
  const [activeSearchIndex, setActiveSearchIndex] = useState<number>(0);

  // File drag-and-drop & replace pending file state (Phase 7)
  const [pendingNewFile, setPendingNewFile] = useState<File | null>(null);
  const [isDraggingFile, setIsDraggingFile] = useState<boolean>(false);

  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [loadingMessage, setLoadingMessage] = useState<string>("Loading document...");
  const [error, setError] = useState<string | null>(null);
  const [showInfoModal, setShowInfoModal] = useState<boolean>(false);

  const viewportContainerRef = useRef<HTMLDivElement | null>(null);
  const fileInputRef = useRef<HTMLInputElement | null>(null);
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

  const loadBarrierRef = useRef<Promise<void>>(Promise.resolve());
  const loadDocument = useCallback(
    async (
      bytes: Uint8Array,
      docFilename: string,
      docSize: number,
      initialPage = 1,
      isHistoryRestore = false,
    ) => {
      const previousLoad = loadBarrierRef.current;
      let releaseLoad!: () => void;
      loadBarrierRef.current = new Promise<void>((resolve) => { releaseLoad = resolve; });
      await previousLoad;
      setIsLoading(true);
      setError(null);
      setSecurityInfo(null);
      setNativeEngineState("READY");
      setEvidenceLevel("UNTRUSTED");
      setSelectedItem(null);
      setLoadingMessage("Parsing PDF structure & inspecting page objects...");

      try {
        // 1. Open StarPDF first so security policy is established
        let starDoc: StarPdfDocumentHandle | null = null;
        try {
          starDoc = await StarPdfClient.open(bytes);
          const detectedSecurity = await starDoc.getSecurityInfo();
          const documentInfo = await starDoc.getInfo();
          setSecurityInfo(detectedSecurity);
          setEvidenceLevel(
            documentInfo.xref_status === "UNRECOVERABLE" ? "UNTRUSTED" :
            documentInfo.xref_status === "RECOVERED_MALFORMED_PREV" ? "RECOVERED_VERIFIED" :
            documentInfo.recovery_status && documentInfo.recovery_status !== "NONE" ? "INFERRED" : "VERIFIED",
          );
          if (starDoc.xrefStatus === "RECOVERED_MALFORMED_PREV") {
            setNativeEngineState("RECOVERED_MALFORMED_PREV");
          }
          if (detectedSecurity.encryption_state !== "NOT_ENCRYPTED") {
            await starDoc.close();
            const friendly = formatPdfErrorMessage("STANDARD_SECURITY_DETECTED");
            setError(friendly.userMessage);
            setIsLoading(false);
            toast.error(friendly.userMessage);
            return;
          }
        } catch {
          if (starDoc) {
            await starDoc.close();
            starDoc = null;
          }
          setNativeEngineState("READ_ONLY");
          setEvidenceLevel("UNTRUSTED");
          setEditorMode("SELECT");
          console.warn("StarPDF native mutation engine unavailable; opening a read-only preview.");
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

        sourceBytesRef.current = bytes;
        inspectionResultRef.current = inspected;
        setSourceBytes(bytes);
        setFilename(docFilename);
        setInspectionResult(inspected);
        setStarPdfDoc(starDoc);
        starPdfDocRef.current = starDoc;
        setPdfProxy(proxy);
        const validPage = Math.max(1, Math.min(initialPage, inspected.metadata.pageCount || 1));
        setCurrentPage(validPage);
        if (!isHistoryRestore) setScale(1.0);
        setFieldValues(initialValues);
        fieldValuesRef.current = initialValues;
        setAnnotationValues(initialAnnotationValues);
        annotationValuesRef.current = initialAnnotationValues;
        setSelectedPages(new Set([validPage]));
        setSearchQuery("");
        setSearchResults([]);
        setActiveSearchIndex(0);
        setIsLoading(false);

        if (!isHistoryRestore) {
          setIsModified(false);
          const initialHistory = createInitialHistoryState(bytes, "Initial document");
          setHistoryState(initialHistory);
          setSurface("workspace");
          void addRecentDocument({
            name: docFilename,
            bytes,
            pageCount: inspected.metadata.pageCount,
          });
          if (onDocumentLoaded) {
            onDocumentLoaded({
              filename: docFilename,
              bytes,
              pageCount: inspected.metadata.pageCount,
            });
          }
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
      } finally {
        releaseLoad();
      }
    },
    [cleanupProxy],
  );

  const handleFileSelected = useCallback(
    async (file: File) => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      await loadDocument(bytes, file.name, file.size, 1);
    },
    [loadDocument],
  );

  useEffect(() => {
    if (initialFile && initialFile.bytes.byteLength > 0) {
      void loadDocument(initialFile.bytes, initialFile.filename, initialFile.bytes.byteLength, 1);
    }
  }, [initialFile, loadDocument]);

  /**
   * Centralized Command Execution Lifecycle
   * Coordinates validation, busy state, execution, atomic document refresh,
   * bounded history, dirty state, selection resolution, error translation, and user feedback.
   */
  const isBusyRef = useRef(false);

  const executeCommand = useCallback(
    async (command: SmartPdfCommand): Promise<void> => {
      if (!starPdfDocRef.current || (command.isMutating && securityInfo?.mutation_allowed !== true)) {
        toast.error(
          "This document is open as a read-only preview because its structure could not be safely loaded for editing.",
        );
        return;
      }
      const isLightweight = command.id === "form.set_value";


      if (!isLightweight) {
        // Page mutations may still be reopening the PDF when the user exports.
        // Keep commands serialized until that load has completed.
        let waitCount = 0;
        while (isBusyRef.current && waitCount < 600) {
          await new Promise((resolve) => setTimeout(resolve, 50));
          waitCount++;
        }

        if (isBusyRef.current) {
          toast.error("The previous PDF operation is still running. Please try again.");
          return;
        }

        isBusyRef.current = true;
        setCommandState({
          status: "RUNNING",
          commandId: command.id,
          label: command.label,
        });
      }

      const context: SmartPdfCommandContext = {
        sourceBytes: sourceBytesRef.current,
        filename,
        currentPage,
        pageCount: inspectionResultRef.current?.metadata.pageCount || 1,
        selection: selectedItem,
        starPdfDoc: starPdfDocRef.current,
        fieldValues: fieldValuesRef.current,
        annotationValues: annotationValuesRef.current,
        inspectionResult: inspectionResultRef.current,
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
          if (command.id === "document.export") {
            setIsModified(false);
          }
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
            fieldValuesRef.current = { ...fieldValuesRef.current, ...updatedFields };
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
            annotationValuesRef.current = { ...annotationValuesRef.current, ...updatedAnnots };
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
        if (
          friendly.technicalDetails?.includes("TEXT_MOVE_DOWNSTREAM_DEPENDENT_TEXT") ||
          friendly.userMessage.includes("nearby text shares its positioning")
        ) {
          toast(
            (t) => (
              <div className="flex flex-col gap-1.5 max-w-sm">
                <span className="text-xs font-semibold text-slate-800 dark:text-slate-100">
                  {friendly.userMessage}
                </span>
                {friendly.technicalDetails && (
                  <span className="text-[11px] text-slate-500 dark:text-slate-400">
                    {friendly.technicalDetails}
                  </span>
                )}
                <div className="flex items-center justify-end gap-2 pt-1 border-t border-slate-100 dark:border-slate-800">
                  <button
                    type="button"
                    data-testid="toast-add-new-text-btn"
                    className="px-2.5 py-1 text-xs font-medium text-white bg-sky-600 hover:bg-sky-700 rounded-md transition-colors shadow-sm"
                    onClick={() => {
                      toast.dismiss(t.id);
                      setEditorMode("FILL_AND_SIGN");
                      setFillAndSignTool("text");
                    }}
                  >
                    Add new text instead
                  </button>
                </div>
              </div>
            ),
            {
              id: "text-move-dependency-toast",
              duration: 6000,
              icon: "⚠️",
            },
          );
        } else {
          toast.error(friendly.userMessage);
        }
      } finally {
        if (!isLightweight) {
          isBusyRef.current = false;
          setCommandState({ status: "IDLE" });
        }
      }
    },

    [
      currentPage,
      filename,
      inspectionResult,
      loadDocument,
      pageGraphics,
      pageImages,
      pageTextSpans,
      selectedItem,
      securityInfo,
      sourceBytes,
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
      const doc = starPdfDocRef.current || starPdfDoc;
      if (!doc || !query.trim()) {
        setSearchResults([]);
        setActiveSearchIndex(0);
        return;
      }

      try {
        const hits = await doc.search(query, { caseSensitive: false });
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

  const handleSelectPageRange = useCallback((startPage: number, endPage: number) => {
    const min = Math.min(startPage, endPage);
    const max = Math.max(startPage, endPage);
    const newSet = new Set<number>();
    for (let p = min; p <= max; p++) {
      newSet.add(p);
    }
    setSelectedPages(newSet);
  }, []);

  const handleSelectAllPages = useCallback(() => {
    if (!inspectionResult) return;
    const total = inspectionResult.metadata.pageCount;
    setSelectedPages(new Set(Array.from({ length: total }, (_, i) => i + 1)));
  }, [inspectionResult]);

  const handleClearSelectedPages = useCallback(() => {
    setSelectedPages(new Set());
  }, []);

  const handleReorderPages = useCallback(
    async (movingPages: number[], targetPage: number, placeBefore: boolean) => {
      if (!inspectionResult) return;
      const movingZero = movingPages.map((p) => p - 1);
      const targetZero = targetPage - 1;
      const cmd = ReorderPagesCommand.fromMoveBlock(
        inspectionResult.metadata.pageCount,
        movingZero,
        targetZero,
        placeBefore,
      );
      await executeCommand(cmd);
    },
    [executeCommand, inspectionResult],
  );

  const handleDuplicateSelected = useCallback(async () => {
    if (selectedPages.size > 1) {
      const zeroIndices = Array.from(selectedPages).map((p) => p - 1);
      await executeCommand(new DuplicatePagesBatchCommand(zeroIndices));
    } else {
      await executeCommand(new DuplicatePageCommand(currentPage - 1));
    }
  }, [currentPage, executeCommand, selectedPages]);

  const handleDeleteSelected = useCallback(async () => {
    if (!inspectionResult) return;
    if (selectedPages.size > 1) {
      if (selectedPages.size >= inspectionResult.metadata.pageCount) {
        toast.error("Cannot delete all pages in a document. At least one page must remain.");
        return;
      }
      const zeroIndices = Array.from(selectedPages).map((p) => p - 1);
      await executeCommand(new DeletePagesBatchCommand(zeroIndices));
    } else {
      if (inspectionResult.metadata.pageCount <= 1) {
        toast.error("Cannot delete the only page in a document.");
        return;
      }
      await executeCommand(new DeletePageCommand(currentPage - 1));
    }
  }, [currentPage, executeCommand, inspectionResult, selectedPages]);

  const handleExtractSelected = useCallback(async () => {
    const pages = selectedPages.size > 1 ? Array.from(selectedPages) : [currentPage];
    const zeroIndices = pages.map((p) => p - 1);
    await executeCommand(new ExtractPagesCommand(zeroIndices));
  }, [currentPage, executeCommand, selectedPages]);

  const handleImportPages = useCallback(
    async (
      importedBytes: Uint8Array,
      selectedPageIndices: number[],
      position: "start" | "end" | "before" | "after",
      sourceFilename: string,
    ) => {
      await executeCommand(
        new InsertImportedPagesCommand(
          importedBytes,
          selectedPageIndices,
          position,
          currentPage - 1,
          sourceFilename,
        ),
      );
    },
    [currentPage, executeCommand],
  );

  const handleSplitRanges = useCallback(
    async (ranges: { start: number; endExclusive: number }[]) => {
      await executeCommand(new SplitDocumentCommand(ranges));
    },
    [executeCommand],
  );

  const performOpenNewFile = useCallback(() => {
    cleanupProxy();
    setSourceBytes(null);
    sourceBytesRef.current = null;
    setFilename("");
    setInspectionResult(null);
    inspectionResultRef.current = null;
    setFieldValues({});
    fieldValuesRef.current = {};
    setAnnotationValues({});
    annotationValuesRef.current = {};
    setIsModified(false);
    setSelectedPages(new Set([1]));
    setError(null);
    setSecurityInfo(null);
    setCurrentPage(1);
    setScale(1.0);
    setIsSearchOpen(false);
    setSearchQuery("");
    setSearchResults([]);
    setHistoryState(createInitialHistoryState(new Uint8Array(0)));
    setSelectedItem(null);
    setEditorMode("SELECT");
    setFillAndSignTool("text");
    setSignatureFile(null);
    setShowFlatFormBanner(true);

    if (pendingNewFile) {
      const fileToLoad = pendingNewFile;
      setPendingNewFile(null);
      void handleFileSelected(fileToLoad);
    } else {
      fileInputRef.current?.click();
    }
  }, [cleanupProxy, handleFileSelected, pendingNewFile]);

  const handleOpenNewFileClick = useCallback(() => {
    if (isModified) {
      setPendingNewFile(null);
      setShowConfirmOpenModal(true);
    } else {
      performOpenNewFile();
    }
  }, [isModified, performOpenNewFile]);

  // Creation & Placement Handlers
  const handlePlaceFreeText = useCallback(
    async (pageIndex: number, x: number, y: number, text: string) => {
      await executeCommand(new AddFreeTextCommand(pageIndex, x, y, text, 12, [0, 0, 0]));
      setEditorMode("SELECT");
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
      await executeCommand(new MoveTextCommand(spanId, dx, dy, pageIndex + 1));
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
      if (freeTextSessionRef.current?.annotationId === annotId) {
        await freeTextCommitRef.current;
        setSelectedItem(null);
        freeTextSessionRef.current = null;
        setFreeTextSession(null);
      }
      await executeCommand(new DeleteAnnotationCommand(currentPage - 1, annotId));
    },
    [currentPage, executeCommand],
  );


  const handleFormFieldChange = useCallback(
    (fieldName: string, value: string | boolean | string[]) => {
      if (!starPdfDoc) {
        toast.error("Form editing is disabled while this document is open as a read-only preview.");
        return;
      }
      if (inspectionResult) {
        const field = inspectionResult.fields.find((f) => f.name === fieldName);
        if (field?.isReadOnly) {
          toast.error("Cannot edit read-only form field.");
          return;
        }
      }
      fieldValuesRef.current = { ...fieldValuesRef.current, [fieldName]: value };
      setFieldValues((prev) => ({ ...prev, [fieldName]: value }));
      setIsModified(true);
      if (sourceBytes) {
        setHistoryState((prev) =>
          pushHistorySnapshot(prev, sourceBytes, `Update field "${fieldName}"`),
        );
      }
    },
    [inspectionResult, sourceBytes, starPdfDoc],
  );

  const handleAnnotationChange = useCallback(
    async (annotId: string, value: string) => {
      await executeCommand(new UpdateAnnotationCommand(annotId, value, currentPage - 1));
    },
    [currentPage, executeCommand],
  );

  const handleUpdateAnnotationProperties = useCallback(
    async (annotId: string, properties: import("@/lib/pdf/starpdf-types").StarPdfUpdateAnnotationInput) => {
      try {
        await executeCommand(new UpdateAnnotationPropertiesCommand(annotId, properties, currentPage - 1));
      } catch (err) {
        const errMsg = err instanceof Error ? err.message : String(err);
        toast.error(`Failed to style annotation: ${errMsg}`);
      }
    },
    [currentPage, executeCommand],
  );

  // Keep a FreeText interaction in memory until it ends. The draft drives the
  // on-page preview; StarPDF receives one serialized transaction on commit.
  useEffect(() => {
    if (freeTextSessionRef.current || selectedItem?.type !== "annotation" ||
        selectedItem.data.subtype !== "FreeText" || !sourceBytesRef.current ||
        selectedItem.data.objectNumber === undefined ||
        selectedItem.data.generationNumber === undefined) return;
    const originalState = freeTextStateFromAnnotation(selectedItem.data);
    const session: FreeTextEditSession = {
      id: ++sessionNumberRef.current,
      annotationId: selectedItem.id,
      pageIndex: selectedItem.pageIndex,
      objectNumber: selectedItem.data.objectNumber,
      generationNumber: selectedItem.data.generationNumber,
      originalState,
      draftState: cloneFreeTextState(originalState),
      lastAppliedState: cloneFreeTextState(originalState),
      originalBytes: sourceBytesRef.current.slice(),
      latestBytes: sourceBytesRef.current.slice(),
      originalIsModified: isModified,
      revision: 0,
      appliedRevision: 0,
      refreshedRevision: 0,
      dirty: false,
      syncing: false,
    };
    freeTextSessionRef.current = session;
    setFreeTextSession({ ...session });
  }, [selectedItem, isModified]);

  const updateFreeTextDraft = useCallback((draft: FreeTextEditState) => {
    const session = freeTextSessionRef.current;
    if (!session || freeTextStatesEqual(session.draftState, draft)) return;
    if (!Number.isFinite(draft.fontSize) || draft.fontSize < 6 || draft.fontSize > 144) return;
    session.draftState = cloneFreeTextState(draft);
    session.revision += 1;
    session.dirty = !freeTextStatesEqual(session.originalState, draft);
    setFreeTextSession({ ...session });
    setIsModified(session.dirty || session.originalIsModified);
  }, []);

  const commitFreeTextSession = useCallback(async () => {
    if (freeTextCommitRef.current) return freeTextCommitRef.current;
    const session = freeTextSessionRef.current;
    if (!session) return;
    const task = (async () => {
      if (session.dirty) {
        const doc = starPdfDocRef.current;
        if (!doc) throw new Error("This text can no longer be edited.");
        // A draft may advance while an async font registration or export runs.
        // Only the final revision can replace the document visible to the user.
        while (freeTextSessionNeedsSync(session)) {
          const revision = session.revision;
          const draft = cloneFreeTextState(session.draftState);
          if (session.appliedRevision < revision) {
            await doc.updateAnnotation(
              session.objectNumber,
              session.generationNumber,
              freeTextStateToUpdatePatch(session.lastAppliedState, draft),
              draft.contents,
            );
            session.latestBytes = await doc.exportIncremental();
            session.lastAppliedState = draft;
            session.appliedRevision = revision;
          }
          if (session.revision === revision) session.refreshedRevision = revision;
        }
        if (freeTextSessionRef.current !== session) return;
        // Linearize the session before the document refresh begins. A later UI
        // event cannot advance a draft that has already been committed.
        freeTextSessionRef.current = null;
        setFreeTextSession(null);
        await loadDocument(session.latestBytes, filename, session.latestBytes.byteLength,
          session.pageIndex + 1, true);
        if (session.dirty) {
          const nextHistory = pushHistorySnapshot(historyStateRef.current, session.latestBytes, "Edit text annotation");
          historyStateRef.current = nextHistory;
          setHistoryState(nextHistory);
          setIsModified(true);
        } else {
          setIsModified(session.originalIsModified);
        }
      }
      if (freeTextSessionRef.current === session) {
        freeTextSessionRef.current = null;
        setFreeTextSession(null);
      }
    })();
    freeTextCommitRef.current = task;
    try { await task; }
    catch (error) { toast.error(formatPdfErrorMessage(error).userMessage); throw error; }
    finally { if (freeTextCommitRef.current === task) freeTextCommitRef.current = null; }
  }, [filename, loadDocument]);

  const cancelFreeTextSession = useCallback(async () => {
    const session = freeTextSessionRef.current;
    if (!session) { setSelectedItem(null); return; }
    freeTextSessionRef.current = null;
    setFreeTextSession(null);
    await freeTextCommitRef.current?.catch(() => undefined);
    if (session.appliedRevision > 0) {
      await loadDocument(session.originalBytes, filename, session.originalBytes.byteLength,
        session.pageIndex + 1, true);
    }
    setIsModified(session.originalIsModified);
    setSelectedItem(null);
  }, [filename, loadDocument]);

  const selectPdfItem = useCallback((item: SmartPdfSelection) => {
    const identified = item ? { ...item, nodeId: selectionNodeId(item, filename) ?? undefined } as SmartPdfSelection : null;
    const session = freeTextSessionRef.current;
    if (session && (item?.type !== "annotation" || item.id !== session.annotationId)) {
      void commitFreeTextSession().then(() => setSelectedItem(identified)).catch(() => undefined);
    } else {
      setSelectedItem(identified);
    }
  }, [commitFreeTextSession, filename]);

  const navigatePage = useCallback((page: number) => {
    void commitFreeTextSession().then(() => handlePageNavigation(page)).catch(() => undefined);
  }, [commitFreeTextSession, handlePageNavigation]);

  const previewAnnotations = useMemo(() => {
    const annotations = inspectionResult?.annotations ?? [];
    if (!freeTextSession) return annotations;
    return annotations.map((annotation) => annotation.id === freeTextSession.annotationId
      ? Object.assign(annotationWithFreeTextState(annotation, freeTextSession.draftState), { liveDraft: true })
      : annotation);
  }, [inspectionResult, freeTextSession]);

  // Phase 7 Unified Global Keyboard Shortcuts (with strict typing safety)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null;
      const isTyping =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable ||
          Boolean(target.closest("[contenteditable='true']")));

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen((open) => !open);
        setPaletteQuery("");
        setPaletteIndex(0);
        return;
      }

      // 1. Escape shortcut: cancel active dialogs, mode, search, and selection (works globally and inside inputs)
      if (e.key === "Escape") {
        e.preventDefault();
        if (paletteOpen) { setPaletteOpen(false); return; }
        target?.blur();
        if (freeTextSessionRef.current) {
          void cancelFreeTextSession();
          return;
        }
        setShowInfoModal(false);
        setShowImportModal(false);
        setShowSplitModal(false);
        setShowConfirmOpenModal(false);
        if (isSearchOpen) {
          setIsSearchOpen(false);
          setSearchQuery("");
        }
        setSelectedItem(null);
        if (editorMode !== "SELECT") {
          setEditorMode("SELECT");
        }
        return;
      }

      // 3. Export / Save shortcut (Cmd/Ctrl + S)
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void commitFreeTextSession().then(() => executeCommand(new ExportDocumentCommand("editable")));
        return;
      }

      // If user is typing inside text inputs/contenteditable, suppress all other editor shortcuts
      if (isTyping) {
        return;
      }

      // 2. Search shortcut (Cmd/Ctrl + F)
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setIsSearchOpen(true);
        setTimeout(() => {
          const searchInput = document.querySelector<HTMLInputElement>(
            '[data-testid="search-query-input"]',
          );
          if (searchInput) {
            searchInput.focus();
            searchInput.select();
          }
        }, 50);
        return;
      }

      // 4. Undo / Redo shortcuts (Cmd/Ctrl + Z, Cmd/Ctrl + Shift + Z, Ctrl + Y)
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        if (e.shiftKey) {
          e.preventDefault();
          void commitFreeTextSession().then(handleRedo);
        } else {
          e.preventDefault();
          void commitFreeTextSession().then(handleUndo);
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        void commitFreeTextSession().then(handleRedo);
        return;
      }

      // 5. Delete / Backspace shortcut for selected objects
      if (e.key === "Delete" || e.key === "Backspace") {
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
        return;
      }

      // 6. Arrow / Page navigation
      if (e.key === "ArrowLeft" || e.key === "PageUp") {
        if (currentPage > 1) {
          e.preventDefault();
          navigatePage(currentPage - 1);
        }
      } else if (e.key === "ArrowRight" || e.key === "PageDown") {
        const totalPages = inspectionResult?.metadata.pageCount || 1;
        if (currentPage < totalPages) {
          e.preventDefault();
          navigatePage(currentPage + 1);
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    currentPage,
    cancelFreeTextSession,
    commitFreeTextSession,
    editorMode,
    executeCommand,
    handleDeleteAnnotation,
    handleFormFieldChange,
    navigatePage,
    handleRedo,
    handleUndo,
    inspectionResult,
    isSearchOpen,
    paletteOpen,
    selectedItem,
    showConfirmOpenModal,
    showImportModal,
    showInfoModal,
    showSplitModal,
  ]);


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


  // 3-Surface Navigation: Home, All Tools, Workspace
  if (!sourceBytes || !inspectionResult) {
    if (surface === "tools") {
      return (
        <div
          className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
          data-testid="smartpdf-editor-workspace"
        >
          <SmartPdfNavbar
            surface="tools"
            onSelectSurface={setSurface}
            tabs={[]}
            activeTabId={null}
            onSelectTab={() => {}}
            onCloseTab={() => {}}
            onOpenNewFile={handleOpenNewFileClick}
          />
          <SmartPdfAllTools
            hasActiveDocument={false}
            onOpenFileWithTool={async (file, tool) => {
              await handleFileSelected(file);
              if (tool.recommendedMode) {
                setEditorMode(tool.recommendedMode);
              }
              setSurface("workspace");
            }}
          />
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

    return (
      <div
        className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
        data-testid="smartpdf-editor-workspace"
      >
        <SmartPdfNavbar
          surface="home"
          onSelectSurface={setSurface}
          tabs={[]}
          activeTabId={null}
          onSelectTab={() => {}}
          onCloseTab={() => {}}
          onOpenNewFile={handleOpenNewFileClick}
        />
        <SmartPdfHome
          onOpenFile={(file, mode) => {
            void handleFileSelected(file);
            if (mode) setEditorMode(mode);
          }}
          onOpenBytes={(name, bytes, mode) => {
            void loadDocument(bytes, name, bytes.byteLength, 1);
            if (mode) setEditorMode(mode);
          }}
          onNavigateToTools={() => setSurface("tools")}
        />
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

  // Active document loaded: surface switching support
  if (surface === "home") {
    return (
      <div
        className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
        data-testid="smartpdf-editor-workspace"
      >
        <SmartPdfNavbar
          surface="home"
          onSelectSurface={setSurface}
          tabs={[
            {
              id: "active-doc",
              title: filename,
              sourceBytes,
              isModified,
              pageCount: inspectionResult.metadata.pageCount,
            },
          ]}
          activeTabId="active-doc"
          onSelectTab={() => setSurface("workspace")}
          onCloseTab={() => {
            cleanupProxy();
            setSourceBytes(null);
            setInspectionResult(null);
            setFilename("");
            setSurface("home");
          }}
          onOpenNewFile={handleOpenNewFileClick}
        />
        <SmartPdfHome
          onOpenFile={(file, mode) => {
            void handleFileSelected(file);
            if (mode) setEditorMode(mode);
          }}
          onOpenBytes={(name, bytes, mode) => {
            void loadDocument(bytes, name, bytes.byteLength, 1);
            if (mode) setEditorMode(mode);
          }}
          onNavigateToTools={() => setSurface("tools")}
        />
      </div>
    );
  }

  if (surface === "tools") {
    return (
      <div
        className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
        data-testid="smartpdf-editor-workspace"
      >
        <SmartPdfNavbar
          surface="tools"
          onSelectSurface={setSurface}
          tabs={[
            {
              id: "active-doc",
              title: filename,
              sourceBytes,
              isModified,
              pageCount: inspectionResult.metadata.pageCount,
            },
          ]}
          activeTabId="active-doc"
          onSelectTab={() => setSurface("workspace")}
          onCloseTab={() => {
            cleanupProxy();
            setSourceBytes(null);
            setInspectionResult(null);
            setFilename("");
            setSurface("home");
          }}
          onOpenNewFile={handleOpenNewFileClick}
        />
        <SmartPdfAllTools
          hasActiveDocument={true}
          onOpenFileWithTool={async (file, tool) => {
            await handleFileSelected(file);
            if (tool.recommendedMode) {
              setEditorMode(tool.recommendedMode);
            }
            setSurface("workspace");
          }}
          onActivateToolForCurrentDoc={(tool) => {
            if (tool.recommendedMode) {
              setEditorMode(tool.recommendedMode);
            } else if (tool.dialogTarget === "merge") {
              mergeInputRef.current?.click();
            } else if (tool.dialogTarget === "split") {
              setShowSplitModal(true);
            } else if (tool.dialogTarget === "info") {
              setShowInfoModal(true);
            }
            setSurface("workspace");
          }}
        />
      </div>
    );
  }

  const isBusy = commandState.status === "RUNNING";
  const nativeMutationEnabled = starPdfDoc !== null && securityInfo?.mutation_allowed === true;
  const paletteResults = searchPdfPalette(paletteQuery);
  const runPaletteCommand = (id: PdfPaletteCommandId) => {
    setPaletteOpen(false);
    switch (id) {
      case "add-text": setEditorMode("FILL_AND_SIGN"); setFillAndSignTool("text"); break;
      case "add-image": imageInputRef.current?.click(); break;
      case "search": setIsSearchOpen(true); break;
      case "delete":
        if (selectedItem?.type === "annotation") void handleDeleteAnnotation(selectedItem.id);
        else if (selectedItem?.type === "image") void executeCommand(new RemoveImageCommand(selectedItem.id));
        else if (selectedItem?.type === "vector") void executeCommand(new DeleteVectorCommand(selectedItem.id));
        else if (selectedItem?.type === "text") void executeCommand(new DeleteTextCommand(selectedItem.id));
        else void handleDeleteSelected();
        break;
      case "duplicate": void handleDuplicateSelected(); break;
      case "rotate": setViewRotation((rotation) => (rotation + 90) % 360); break;
      case "extract": void handleExtractSelected(); break;
      case "properties": setShowInfoModal(true); break;
      case "bookmarks": toast("Bookmark navigation is not available for this document."); break;
      case "attachments": toast("Attachment browsing is not available for this document."); break;
      case "export": void commitFreeTextSession().then(() => executeCommand(new ExportDocumentCommand("editable"))); break;
    }
  };

  return (
    <div
      className="flex flex-col h-full w-full bg-slate-100 overflow-hidden relative select-none"
      data-testid="smartpdf-editor-workspace"
    >
      {paletteOpen && (
        <div className="fixed inset-0 z-[100] flex justify-center bg-slate-950/35 pt-[12vh]" onMouseDown={(event) => { if (event.target === event.currentTarget) setPaletteOpen(false); }}>
          <div role="dialog" aria-label="Commands" className="h-fit w-[min(92vw,460px)] rounded-xl border border-slate-200 bg-white p-2 shadow-2xl" data-testid="smartpdf-command-palette">
            <input autoFocus value={paletteQuery} onChange={(event) => { setPaletteQuery(event.target.value); setPaletteIndex(0); }}
              onKeyDown={(event) => {
                if (event.key === "ArrowDown") { event.preventDefault(); setPaletteIndex((index) => Math.min(index + 1, paletteResults.length - 1)); }
                if (event.key === "ArrowUp") { event.preventDefault(); setPaletteIndex((index) => Math.max(index - 1, 0)); }
                if (event.key === "Enter" && paletteResults[paletteIndex]) { event.preventDefault(); runPaletteCommand(paletteResults[paletteIndex].id); }
              }}
              placeholder="Search commands…" aria-label="Search commands" data-testid="smartpdf-command-query"
              className="w-full rounded-lg border border-slate-200 px-3 py-2 text-sm outline-none focus:border-sky-500" />
            <div role="listbox" aria-label="Matching commands" className="max-h-80 overflow-auto py-1">
              {paletteResults.map((command, index) => <button key={command.id} type="button" role="option"
                aria-selected={index === paletteIndex} data-testid={`smartpdf-command-${command.id}`}
                onClick={() => runPaletteCommand(command.id)}
                className={`block w-full rounded-md px-3 py-2 text-left text-sm ${index === paletteIndex ? "bg-sky-50 text-sky-900" : "text-slate-700 hover:bg-slate-50"}`}>{command.label}</button>)}
              {!paletteResults.length && <p className="px-3 py-3 text-sm text-slate-500">No matching commands</p>}
            </div>
          </div>
        </div>
      )}
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

      {nativeEngineState === "RECOVERED_MALFORMED_PREV" ? (
        <div
          className="border-b border-sky-300 bg-sky-50 px-4 py-2.5 text-sm text-sky-950 flex items-center gap-2 shrink-0"
          data-testid="starpdf-xref-recovered-note"
          role="status"
        >
          <ShieldCheck className="size-4 text-sky-700 shrink-0" />
          <span>
            StarPDF safely recovered a malformed document history link. Editing is enabled, and export will write a clean current cross-reference section.
          </span>
        </div>
      ) : null}

      {nativeEngineState === "READ_ONLY" ? (
        <div
          className="border-b border-amber-300 bg-amber-50 px-4 py-2.5 text-sm text-amber-950 flex items-center gap-2 shrink-0"
          data-testid="starpdf-read-only-note"
          role="status"
        >
          <AlertTriangle className="size-4 text-amber-700 shrink-0" />
          <span>
            Read-only preview: this PDF can be displayed, but its structure could not be safely loaded by the editing engine. Native editing and mutation controls are disabled.
          </span>
        </div>
      ) : null}

      {evidenceLevel === "INFERRED" && (
        <div role="status" data-testid="starpdf-inferred-note" className="border-b border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-950">
          Some document structure was inferred during recovery. You can view and search this PDF; editing is paused because the affected objects cannot be verified safely.
        </div>
      )}
      {evidenceLevel === "UNTRUSTED" && nativeEngineState !== "READ_ONLY" && (
        <div role="status" data-testid="starpdf-untrusted-note" className="border-b border-amber-300 bg-amber-50 px-4 py-2 text-sm text-amber-950">
          This document cannot be verified for editing. It is open for viewing only.
        </div>
      )}

      {/* Flat Form Suggestion Banner */}
      {nativeMutationEnabled && showFlatFormBanner && isFlatForm && (
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
        canUndo={nativeMutationEnabled && canUndo(historyState)}
        canRedo={nativeMutationEnabled && canRedo(historyState)}
        onUndo={() => void commitFreeTextSession().then(handleUndo)}
        onRedo={() => void commitFreeTextSession().then(handleRedo)}
        isSearchOpen={isSearchOpen}
        onToggleSearch={setIsSearchOpen}
        searchQuery={searchQuery}
        searchResultCount={searchResults.length}
        activeSearchIndex={activeSearchIndex}
        onSearchChange={(q) => void handleSearchChange(q)}
        onNextSearchResult={handleNextSearchResult}
        onPrevSearchResult={handlePrevSearchResult}
        onPageChange={navigatePage}
        onZoomIn={handleZoomIn}
        onZoomOut={handleZoomOut}
        onFitWidth={handleFitWidth}
        onFitPage={handleFitPage}
        onExport={async (mode) => {
          await commitFreeTextSession();
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
        mutationEnabled={nativeMutationEnabled}
        onHomeClick={() => {
          if (externalHomeClick) externalHomeClick();
          else setSurface("home");
        }}
        onAllToolsClick={() => {
          if (externalAllToolsClick) externalAllToolsClick();
          else setSurface("tools");
        }}
      />


      {/* Page Operations Rail / Actions */}
      <PdfPageOperations
        currentPage={currentPage}
        pageCount={inspectionResult.metadata.pageCount}
        selectedCount={selectedPages.size}
        isProcessing={isBusy}
        onDelete={handleDeleteSelected}
        onMoveLeft={() => {
          if (currentPage <= 1) return;
          void executeCommand(new MovePageCommand(currentPage - 1, currentPage - 2));
        }}
        onMoveRight={() => {
          if (currentPage >= inspectionResult.metadata.pageCount) return;
          void executeCommand(new MovePageCommand(currentPage - 1, currentPage));
        }}
        onDuplicate={handleDuplicateSelected}
        onInsertBlank={() => {
          const currentPageInfo = inspectionResult.pages[currentPage - 1] || {
            width: 612,
            height: 792,
            rotation: 0,
          };
          void executeCommand(
            new InsertBlankPageCommand(
              currentPage - 1, // insert after current page (0-indexed)
              currentPageInfo.width,
              currentPageInfo.height,
              currentPageInfo.rotation as 0 | 90 | 180 | 270,
            ),
          );
        }}
        onExtract={handleExtractSelected}
        onMergeFiles={async (additions) => {
          await executeCommand(new MergeDocumentsCommand(additions));
        }}
        onImport={() => setShowImportModal(true)}
        onSplit={() => setShowSplitModal(true)}
        mutationEnabled={nativeMutationEnabled}
      />

      {/* Hidden File Input for Opening / Replacing Document */}
      <input
        ref={fileInputRef}
        type="file"
        accept="application/pdf"
        className="hidden"
        aria-label="Open PDF document"
        onChange={async (e) => {
          const files = e.target.files;
          if (files && files.length > 0) {
            await handleFileSelected(files[0]);
          }
          if (fileInputRef.current) fileInputRef.current.value = "";
        }}
      />

      {/* Hidden File Input for Adding/Merging PDFs */}
      <input ref={imageInputRef} type="file" accept="image/*" className="hidden" aria-label="Add image"
        onChange={(event) => {
          const file = event.target.files?.[0];
          if (file && inspectionResult) {
            const page = inspectionResult.pages[currentPage - 1];
            void executeCommand(new AddImageCommand(currentPage - 1,
              (page.width - 180) / 2, (page.height - 120) / 2, 180, 120, file));
          }
          event.target.value = "";
        }} />
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
        {/* Left Collapsible Thumbnail Rail / Page Organizer */}
        {isThumbnailsOpen && (
          <PdfThumbnailRail
            pdfDocument={pdfProxy}
            pageCount={inspectionResult.metadata.pageCount}
            currentPage={currentPage}
            selectedPages={selectedPages}
            onPageSelect={navigatePage}
            onToggleSelection={handleTogglePageSelection}
            onSelectRange={handleSelectPageRange}
            onSelectAllPages={handleSelectAllPages}
            onClearSelectedPages={handleClearSelectedPages}
            onReorderPages={nativeMutationEnabled ? handleReorderPages : undefined}
            onDuplicateSelected={nativeMutationEnabled ? handleDuplicateSelected : undefined}
            onDeleteSelected={nativeMutationEnabled ? handleDeleteSelected : undefined}
            onExtractSelected={nativeMutationEnabled ? handleExtractSelected : undefined}
          />
        )}

        {/* Center Viewport Canvas Surface */}
        <main
          ref={viewportContainerRef}
          className="flex-1 bg-slate-100/90 overflow-auto p-6 sm:p-8 flex items-center justify-center min-w-0 relative"
          aria-label="PDF Document Page Viewport"
          onDragOver={(e) => {
            e.preventDefault();
            e.dataTransfer.dropEffect = "copy";
            setIsDraggingFile(true);
          }}
          onDragLeave={(e) => {
            if (e.currentTarget.contains(e.relatedTarget as Node)) return;
            setIsDraggingFile(false);
          }}
          onDrop={async (e) => {
            e.preventDefault();
            setIsDraggingFile(false);
            const files = e.dataTransfer.files;
            if (files && files.length > 0) {
              const pdfFile = Array.from(files).find(
                (f) => f.type === "application/pdf" || f.name.toLowerCase().endsWith(".pdf"),
              );
              if (pdfFile) {
                if (isModified) {
                  setPendingNewFile(pdfFile);
                  setShowConfirmOpenModal(true);
                } else {
                  await handleFileSelected(pdfFile);
                }
              }
            }
          }}
          onClick={(e) => {
            if (editorMode === "SELECT" &&
                !(e.target as Element).closest('[data-testid="pdf-contextual-toolbar"]')) {
              selectPdfItem(null);
            }
          }}
        >
          {/* Drag & Drop File Upload Overlay */}
          {isDraggingFile && (
            <div
              className="absolute inset-0 z-50 bg-sky-500/10 border-2 border-dashed border-sky-500 backdrop-blur-xs flex flex-col items-center justify-center gap-2 pointer-events-none animate-in fade-in duration-100"
              data-testid="workspace-drop-overlay"
            >
              <FolderOpen className="size-12 text-sky-600 animate-bounce" />
              <span className="text-sm font-semibold text-sky-900 bg-white/95 px-4 py-2 rounded-lg shadow-md">
                Drop PDF here to open in StarPDF
              </span>
            </div>
          )}

          {/* Floating Contextual Object Action Bar */}
          {nativeMutationEnabled && selectedItem && (
            <PdfContextualToolbar
              selection={selectedItem}
              containerRef={viewportContainerRef}
              onDeselect={() => selectPdfItem(null)}
              onReplaceText={async (spanId, newText) => {
                await executeCommand(new ReplaceTextCommand(spanId, newText));
              }}
              onApplyTextStyle={async (spanId, text, patch) => {
                await executeCommand(new ApplyTextStyleCommand(spanId, text, patch));
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
              onUpdateAnnotationProperties={handleUpdateAnnotationProperties}
              freeTextDraft={freeTextSession?.annotationId === selectedItem.id ? freeTextSession.draftState : undefined}
              onFreeTextDraftChange={updateFreeTextDraft}
              onFreeTextDone={() => { void commitFreeTextSession().then(() => setSelectedItem(null)); }}
              onFreeTextCancel={() => { void cancelFreeTextSession(); }}
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
              rotation={((inspectionResult.pages[currentPage - 1]?.rotation || 0) + viewRotation) % 360}
              pageWidth={inspectionResult.pages[currentPage - 1]?.width || 612}
              pageHeight={inspectionResult.pages[currentPage - 1]?.height || 792}
              textSpans={pageTextSpans}
              images={pageImages}
              graphics={pageGraphics}
              fields={inspectionResult.fields}
              annotations={previewAnnotations}
              candidates={currentCandidates}
              formAssistEnabled={nativeMutationEnabled && formAssistEnabled}
              selectedItem={selectedItem}
              onSelectItem={nativeMutationEnabled ? selectPdfItem : undefined}
              onCanvasRendered={handleCanvasRendered}
              mode={nativeMutationEnabled ? editorMode : "SELECT"}
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
              searchResults={searchResults}
              activeSearchIndex={activeSearchIndex}
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
          {isModified ? (
            <>
              <span className="text-slate-300">•</span>
              <span className="text-amber-600 font-medium" data-testid="status-unsaved-changes">Unsaved changes</span>
            </>
          ) : (
            <>
              <span className="text-slate-300">•</span>
              <span className="text-emerald-600 font-medium" data-testid="status-saved">Saved</span>
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

      {/* Multi-Document Import / Merge Modal */}
      <SmartPdfImportModal
        isOpen={showImportModal}
        currentPage={currentPage}
        totalPageCount={inspectionResult.metadata.pageCount}
        onClose={() => setShowImportModal(false)}
        onImportPages={handleImportPages}
      />

      {/* Split / Extract Document Modal */}
      <SmartPdfSplitModal
        isOpen={showSplitModal}
        totalPageCount={inspectionResult.metadata.pageCount}
        selectedPages={selectedPages}
        onClose={() => setShowSplitModal(false)}
        onExtractSelected={handleExtractSelected}
        onSplitRanges={handleSplitRanges}
      />
    </div>
  );
}
