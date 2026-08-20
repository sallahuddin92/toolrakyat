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
import type { StarPdfSearchResult } from "@/lib/pdf/starpdf-types";

import { PdfDropzone } from "./PdfDropzone";
import { PdfToolbar } from "./PdfToolbar";
import { PdfThumbnailRail } from "./PdfThumbnailRail";
import { PdfPageCanvas } from "./PdfPageCanvas";
import { PdfFormInspector } from "./PdfFormInspector";
import { PdfDocumentInfo } from "./PdfDocumentInfo";

export function SmartPdfEditor() {
  const [sourceBytes, setSourceBytes] = useState<Uint8Array | null>(null);
  const [filename, setFilename] = useState<string>("");
  const [inspectionResult, setInspectionResult] = useState<DocumentInspectionResult | null>(null);
  const [pdfProxy, setPdfProxy] = useState<PDFDocumentProxy | null>(null);
  const [starPdfDoc, setStarPdfDoc] = useState<StarPdfDocumentHandle | null>(null);

  const [currentPage, setCurrentPage] = useState<number>(1);
  const [scale, setScale] = useState<number>(1.0);
  const [fieldValues, setFieldValues] = useState<Record<string, string | boolean | string[]>>({});
  const [isModified, setIsModified] = useState<boolean>(false);

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
    async (bytes: Uint8Array, docFilename: string, docSize: number) => {
      setIsLoading(true);
      setError(null);
      setLoadingMessage("Parsing PDF structure & detecting form fields...");

      try {
        // 1. Inspect using pdf-lib (AcroForms, metadata, dimensions)
        const inspected = await inspectPdfDocument(bytes, docFilename, docSize);

        // 2. Open StarPDF WASM document handle for validation & search
        let starDoc: StarPdfDocumentHandle | null = null;
        try {
          starDoc = await StarPdfClient.open(bytes);
        } catch (starErr) {
          console.warn("StarPDF validation note:", starErr);
        }

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
        setCurrentPage(1);
        setScale(1.0);
        setFieldValues(initialValues);
        setIsModified(false);
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

  const handleOpenNewFile = useCallback(() => {
    cleanupProxy();
    setSourceBytes(null);
    setFilename("");
    setInspectionResult(null);
    setFieldValues({});
    setIsModified(false);
    setError(null);
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

      {/* Main Workspace: Thumbnails + Viewport Canvas + Form Inspector */}
      <div className="flex-1 flex overflow-hidden">
        {/* Left Thumbnail Rail */}
        <PdfThumbnailRail
          pdfDocument={pdfProxy}
          pageCount={inspectionResult.metadata.pageCount}
          currentPage={currentPage}
          onPageSelect={setCurrentPage}
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

        {/* Right Form Fields Inspector */}
        <PdfFormInspector
          fields={inspectionResult.fields}
          fieldValues={fieldValues}
          onFieldValueChange={handleFieldValueChange}
          onResetForm={handleResetForm}
          isModified={isModified}
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
