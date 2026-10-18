"use client";

import React, { useState, useEffect, useRef } from "react";
import {
  FolderOpen,
  ShieldCheck,
  Type,
  LayoutGrid,
  FileSpreadsheet,
  MessageSquare,
  FileText,
  Clock,
  Trash2,
  ArrowRight,
  Sparkles,
  UploadCloud,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import toast from "react-hot-toast";
import type { EditorMode } from "../workspace/PdfToolbar";
import type { RecentDocumentMeta } from "../types";
import {
  getRecentDocuments,
  getRecentDocumentBytes,
  removeRecentDocument,
  clearRecentDocuments,
} from "../recent/recentFilesStore";

interface SmartPdfHomeProps {
  onOpenFile: (file: File, targetMode?: EditorMode) => void;
  onOpenBytes: (name: string, bytes: Uint8Array, targetMode?: EditorMode) => void;
  onNavigateToTools: () => void;
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

function formatRelativeTime(timestamp: number): string {
  const diffSeconds = Math.floor((Date.now() - timestamp) / 1000);
  if (diffSeconds < 60) return "Just now";
  const diffMinutes = Math.floor(diffSeconds / 60);
  if (diffMinutes < 60) return `${diffMinutes}m ago`;
  const diffHours = Math.floor(diffMinutes / 60);
  if (diffHours < 24) return `${diffHours}h ago`;
  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return `${diffDays}d ago`;
  return new Date(timestamp).toLocaleDateString();
}

export function SmartPdfHome({
  onOpenFile,
  onOpenBytes,
  onNavigateToTools,
}: SmartPdfHomeProps) {
  const [recentDocs, setRecentDocs] = useState<RecentDocumentMeta[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const pendingModeRef = useRef<EditorMode | undefined>(undefined);

  useEffect(() => {
    void getRecentDocuments().then(setRecentDocs);
  }, []);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file && file.type === "application/pdf") {
      onOpenFile(file, pendingModeRef.current);
    }
    // reset input
    e.target.value = "";
    pendingModeRef.current = undefined;
  };

  const triggerFileSelect = (mode?: EditorMode) => {
    pendingModeRef.current = mode;
    fileInputRef.current?.click();
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(true);
  };

  const handleDragLeave = () => {
    setIsDragging(false);
  };

  const handleDrop = (e: React.DragEvent) => {
    e.preventDefault();
    setIsDragging(false);
    const file = e.dataTransfer.files?.[0];
    if (file && file.name.toLowerCase().endsWith(".pdf")) {
      onOpenFile(file);
    }
  };

  const handleOpenRecent = async (item: RecentDocumentMeta) => {
    try {
      const cachedBytes = await getRecentDocumentBytes(item.id);
      if (cachedBytes && cachedBytes.byteLength > 0) {
        toast.success(`Loaded "${item.name}" from local storage cache.`);
        onOpenBytes(item.name, cachedBytes);
      } else {
        toast.error(
          `Local cache not found for "${item.name}". Please re-select the file from your local disk to reopen.`,
          { duration: 5000, id: "cache-reopen" }
        );
        triggerFileSelect();
      }
    } catch {
      toast.error(`Could not read local cache for "${item.name}". Please re-select the file.`);
      triggerFileSelect();
    }
  };

  const handleRemoveRecent = async (id: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await removeRecentDocument(id);
    setRecentDocs((prev) => prev.filter((d) => d.id !== id));
  };

  const handleClearAllRecent = async () => {
    await clearRecentDocuments();
    setRecentDocs([]);
  };

  const WORKFLOW_CARDS: Array<{
    title: string;
    description: string;
    icon: React.ReactNode;
    mode: EditorMode;
    color: string;
    badge: string;
  }> = [
    {
      title: "Edit Text & Objects",
      description: "Direct in-place text editing, font styling, image manipulation, and vector shapes.",
      icon: <Type className="size-5 text-sky-600" />,
      mode: "TEXT",
      color: "from-sky-50 to-indigo-50 border-sky-200/80 hover:border-sky-300",
      badge: "Text & Graphics",
    },
    {
      title: "Organize & Pages",
      description: "Reorder, rotate (90°/180°), duplicate, delete, extract, and merge document pages.",
      icon: <LayoutGrid className="size-5 text-indigo-600" />,
      mode: "PAGES",
      color: "from-indigo-50 to-purple-50 border-indigo-200/80 hover:border-indigo-300",
      badge: "Page Manager",
    },
    {
      title: "Fill & Sign",
      description: "Interactive AcroForms, flat form detection assist, signature stamps, and checkmarks.",
      icon: <FileSpreadsheet className="size-5 text-emerald-600" />,
      mode: "FILL_AND_SIGN",
      color: "from-emerald-50 to-teal-50 border-emerald-200/80 hover:border-emerald-300",
      badge: "Forms & Stamps",
    },
    {
      title: "Annotate & Markup",
      description: "FreeText comments, callout boxes, color highlights, and freehand ink sketches.",
      icon: <MessageSquare className="size-5 text-amber-600" />,
      mode: "ANNOTATE",
      color: "from-amber-50 to-orange-50 border-amber-200/80 hover:border-amber-300",
      badge: "Review Tools",
    },
  ];

  return (
    <div
      className="flex-1 overflow-y-auto bg-slate-50/70 p-4 sm:p-8 select-none"
      data-testid="smartpdf-home-view"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
    >
      <input
        ref={fileInputRef}
        type="file"
        accept="application/pdf"
        className="hidden"
        onChange={handleFileChange}
      />

      <div className="max-w-6xl mx-auto space-y-8">
        {/* Top Hero Banner */}
        <div className="relative overflow-hidden rounded-3xl border border-slate-200/80 bg-white p-6 sm:p-10 shadow-sm">
          <div className="relative z-10 flex flex-col md:flex-row items-start md:items-center justify-between gap-6">
            <div className="space-y-3 max-w-2xl">
              <div className="inline-flex items-center gap-2 px-3 py-1 rounded-full text-xs font-semibold bg-sky-50 text-sky-700 border border-sky-200/60">
                <Sparkles className="size-3.5" />
                <span>SmartPDF • Precision Document Engineering</span>
              </div>
              <h1 className="text-2xl sm:text-4xl font-extrabold text-slate-900 tracking-tight">
                Professional PDF Workspace
              </h1>
              <p className="text-sm sm:text-base text-slate-600 leading-relaxed">
                Desktop-grade PDF editing with live text modification, lossless page organization, AcroForm filling, and cryptographic security auditing.
              </p>
              <div className="flex items-center gap-2 pt-1 text-xs text-slate-500">
                <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md bg-slate-100 text-slate-700 font-medium border border-slate-200">
                  <ShieldCheck className="size-3.5 text-emerald-600" />
                  100% Client-Side WebAssembly
                </span>
                <span className="text-slate-400">•</span>
                <span>Zero server uploads</span>
              </div>
            </div>

            {/* Quick Action Button Box */}
            <div className="flex flex-col sm:flex-row md:flex-col gap-3 w-full md:w-auto shrink-0">
              <Button
                size="lg"
                onClick={() => triggerFileSelect()}
                className="h-12 px-6 rounded-2xl bg-sky-600 hover:bg-sky-700 text-white font-medium shadow-sm gap-2"
                data-testid="home-open-pdf-btn"
              >
                <FolderOpen className="size-4" />
                <span>Open PDF Document</span>
              </Button>
              <Button
                variant="outline"
                size="lg"
                onClick={onNavigateToTools}
                className="h-12 px-6 rounded-2xl border-slate-300 text-slate-700 hover:bg-slate-100 font-medium gap-2"
                data-testid="home-all-tools-btn"
              >
                <span>Browse All Tools</span>
                <ArrowRight className="size-4" />
              </Button>
            </div>
          </div>

          {/* Drag & Drop Visual Target */}
          <div
            onClick={() => triggerFileSelect()}
            className={`mt-6 rounded-2xl border-2 border-dashed p-6 text-center cursor-pointer transition-colors ${
              isDragging
                ? "border-sky-500 bg-sky-50/70"
                : "border-slate-200 hover:border-sky-400 hover:bg-slate-50/60"
            }`}
          >
            <div className="flex flex-col items-center justify-center gap-2">
              <div className="p-3 rounded-full bg-sky-50 text-sky-600">
                <UploadCloud className="size-6" />
              </div>
              <div className="text-sm font-semibold text-slate-800">
                Drag and drop your PDF here, or click to browse
              </div>
              <div className="text-xs text-slate-500">
                Supports standard PDF documents, forms, scanned pages, and contracts
              </div>
            </div>
          </div>
        </div>

        {/* Workflow Quick-Launch Cards */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-lg font-bold text-slate-900 tracking-tight">
                Start a Professional Workflow
              </h2>
              <p className="text-xs text-slate-500">
                Select a workflow to open a document directly in that editing mode
              </p>
            </div>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
            {WORKFLOW_CARDS.map((card) => (
              <Card
                key={card.title}
                onClick={() => triggerFileSelect(card.mode)}
                className={`cursor-pointer rounded-2xl border bg-gradient-to-br ${card.color} shadow-xs transition-all hover:shadow-md hover:-translate-y-0.5`}
                data-testid={`workflow-card-${card.mode.toLowerCase()}`}
              >
                <CardContent className="p-5 space-y-3">
                  <div className="flex items-center justify-between">
                    <div className="p-2.5 rounded-xl bg-white shadow-xs border border-slate-200/60">
                      {card.icon}
                    </div>
                    <span className="text-[10px] font-semibold uppercase tracking-wider text-slate-500 bg-white/80 px-2 py-0.5 rounded-md border border-slate-200/40">
                      {card.badge}
                    </span>
                  </div>
                  <div>
                    <h3 className="font-semibold text-slate-900 text-sm">{card.title}</h3>
                    <p className="text-xs text-slate-600 mt-1 line-clamp-2 leading-relaxed">
                      {card.description}
                    </p>
                  </div>
                  <div className="pt-1 flex items-center text-xs font-medium text-slate-700 gap-1 group">
                    <span>Launch</span>
                    <ChevronRight className="size-3.5 transition-transform group-hover:translate-x-0.5" />
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>

        {/* Recent Files Section */}
        <div className="space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Clock className="size-4 text-slate-500" />
              <h2 className="text-lg font-bold text-slate-900 tracking-tight">
                Recent Documents
              </h2>
            </div>
            {recentDocs.length > 0 && (
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClearAllRecent}
                className="text-xs text-slate-500 hover:text-red-600 h-8 gap-1"
                data-testid="clear-recent-btn"
              >
                <Trash2 className="size-3.5" />
                <span>Clear Recent</span>
              </Button>
            )}
          </div>

          {recentDocs.length === 0 ? (
            <div className="rounded-2xl border border-slate-200 bg-white p-8 text-center text-slate-400">
              <FileText className="size-8 mx-auto text-slate-300 mb-2" />
              <p className="text-sm font-medium text-slate-600">No recent documents</p>
              <p className="text-xs text-slate-400 mt-1">
                Open a PDF above to work with it. Recent files stay safely on your device.
              </p>
            </div>
          ) : (
            <div className="rounded-2xl border border-slate-200 bg-white overflow-hidden shadow-xs divide-y divide-slate-100">
              {recentDocs.map((item) => (
                <div
                  key={item.id}
                  onClick={() => handleOpenRecent(item)}
                  className="flex items-center justify-between p-4 hover:bg-slate-50/80 cursor-pointer transition-colors group"
                  data-testid={`recent-item-${item.id}`}
                >
                  <div className="flex items-center gap-3.5 min-w-0">
                    <div className="grid size-9 place-items-center rounded-xl bg-red-50 text-red-600 border border-red-100 shrink-0">
                      <FileText className="size-4.5" />
                    </div>
                    <div className="min-w-0">
                      <div className="text-sm font-medium text-slate-900 truncate group-hover:text-sky-600 transition-colors">
                        {item.name}
                      </div>
                      <div className="flex items-center gap-2 text-xs text-slate-400 mt-0.5">
                        <span>{formatFileSize(item.sizeBytes)}</span>
                        {item.pageCount && (
                          <>
                            <span>•</span>
                            <span>{item.pageCount} {item.pageCount === 1 ? "page" : "pages"}</span>
                          </>
                        )}
                        <span>•</span>
                        <span>Opened {formatRelativeTime(item.lastOpened)}</span>
                      </div>
                    </div>
                  </div>

                  <div className="flex items-center gap-2 shrink-0">
                    <Button
                      size="sm"
                      variant="ghost"
                      className="h-8 text-xs text-sky-600 hover:text-sky-700 hover:bg-sky-50 font-medium"
                    >
                      Open
                    </Button>
                    <button
                      type="button"
                      onClick={(e) => handleRemoveRecent(item.id, e)}
                      className="p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 transition-colors"
                      title="Remove from recents"
                      aria-label="Remove from recents"
                    >
                      <Trash2 className="size-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
