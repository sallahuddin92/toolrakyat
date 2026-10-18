"use client";

import React from "react";
import {
  FileText,
  Plus,
  X,
  Home,
  LayoutGrid,
  ShieldCheck,
  FolderOpen,
  MousePointer,
  Type,
  FileSpreadsheet,
  MessageSquare,
  Layers,
  Undo2,
  Redo2,
  ZoomIn,
  ZoomOut,
  Maximize2,
  Download,
  Command,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import type { EditorMode } from "../workspace/PdfToolbar";
import type { SmartPdfSurface, DocumentTabSession } from "../types";

interface SmartPdfNavbarProps {
  surface: SmartPdfSurface;
  onSelectSurface: (surface: SmartPdfSurface) => void;
  tabs: DocumentTabSession[];
  activeTabId: string | null;
  onSelectTab: (tabId: string) => void;
  onCloseTab: (tabId: string, e: React.MouseEvent) => void;
  onOpenNewFile: () => void;

  // Workspace-only controls (optional when surface === 'workspace')
  mode?: EditorMode;
  onModeChange?: (mode: EditorMode) => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
  onZoomIn?: () => void;
  onZoomOut?: () => void;
  onFitPage?: () => void;
  zoomPercent?: number;
  onOpenCommandPalette?: () => void;
  onExport?: () => void;
}

export function SmartPdfNavbar({
  surface,
  onSelectSurface,
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onOpenNewFile,
  mode = "SELECT",
  onModeChange,
  canUndo = false,
  canRedo = false,
  onUndo,
  onRedo,
  onZoomIn,
  onZoomOut,
  onFitPage,
  zoomPercent = 100,
  onOpenCommandPalette,
  onExport,
}: SmartPdfNavbarProps) {
  return (
    <header
      className="h-14 border-b border-slate-200 bg-white px-3 sm:px-4 flex items-center justify-between gap-3 shrink-0 select-none z-30"
      data-testid="smartpdf-global-navbar"
    >
      {/* Left: Branding & Core Navigation Tabs */}
      <div className="flex items-center gap-2 sm:gap-3 min-w-0">
        {/* Brand */}
        <div
          onClick={() => onSelectSurface("home")}
          className="flex items-center gap-2 cursor-pointer shrink-0 group"
          title="Go to SmartPDF Home"
        >
          <div className="grid size-8 place-items-center rounded-xl bg-gradient-to-br from-sky-500 to-indigo-600 text-white font-bold text-xs shadow-xs group-hover:scale-105 transition-transform">
            ✦
          </div>
          <div className="hidden lg:block leading-tight">
            <div className="flex items-center gap-1.5">
              <span className="font-bold text-slate-900 text-sm tracking-tight">SmartPDF</span>
              <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-slate-100 text-slate-600 border border-slate-200">
                StarPDF
              </span>
            </div>
          </div>
        </div>

        <div className="h-5 w-px bg-slate-200 hidden sm:block shrink-0" />

        {/* Surface Tabs: Home & All Tools */}
        <div className="flex items-center gap-1 shrink-0">
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => onSelectSurface("home")}
            className={`h-8 px-2.5 rounded-lg text-xs font-medium gap-1.5 ${
              surface === "home"
                ? "bg-slate-100 text-slate-900"
                : "text-slate-600 hover:text-slate-900 hover:bg-slate-50"
            }`}
            data-testid="navbar-tab-home"
          >
            <Home className="size-3.5" />
            <span className="hidden sm:inline">Home</span>
          </Button>

          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={() => onSelectSurface("tools")}
            className={`h-8 px-2.5 rounded-lg text-xs font-medium gap-1.5 ${
              surface === "tools"
                ? "bg-slate-100 text-slate-900"
                : "text-slate-600 hover:text-slate-900 hover:bg-slate-50"
            }`}
            data-testid="navbar-tab-all-tools"
          >
            <LayoutGrid className="size-3.5" />
            <span className="hidden sm:inline">All Tools</span>
          </Button>
        </div>

        {/* Document Tabs Rail */}
        {tabs.length > 0 && (
          <div className="flex items-center gap-1 max-w-[200px] sm:max-w-[360px] md:max-w-[480px] overflow-x-auto py-1">
            {tabs.map((tab) => {
              const isActive = surface === "workspace" && tab.id === activeTabId;
              return (
                <div
                  key={tab.id}
                  onClick={() => onSelectTab(tab.id)}
                  className={`flex items-center gap-1.5 h-8 px-2.5 rounded-lg text-xs font-medium cursor-pointer transition-all shrink-0 max-w-[160px] border ${
                    isActive
                      ? "bg-sky-50/80 border-sky-200 text-sky-950 shadow-2xs"
                      : "bg-slate-50 border-slate-200/70 text-slate-600 hover:bg-slate-100/70"
                  }`}
                  data-testid={`document-tab-${tab.id}`}
                  title={tab.title}
                >
                  <FileText className={`size-3.5 shrink-0 ${isActive ? "text-sky-600" : "text-slate-400"}`} />
                  <span className="truncate">{tab.title}</span>
                  {tab.isModified && (
                    <span
                      className="size-1.5 rounded-full bg-amber-500 shrink-0"
                      title="Unsaved changes"
                      data-testid="tab-dirty-indicator"
                    />
                  )}
                  <button
                    type="button"
                    onClick={(e) => onCloseTab(tab.id, e)}
                    className="p-0.5 rounded hover:bg-slate-200/80 text-slate-400 hover:text-slate-700 ml-0.5"
                    aria-label={`Close ${tab.title}`}
                    data-testid={`close-tab-${tab.id}`}
                  >
                    <X className="size-3" />
                  </button>
                </div>
              );
            })}
          </div>
        )}

        {/* Plus Open Button */}
        <Button
          type="button"
          variant="ghost"
          size="icon"
          onClick={onOpenNewFile}
          className="size-8 rounded-lg text-slate-500 hover:text-slate-900 shrink-0"
          title="Open another PDF document"
          data-testid="navbar-open-new-file"
        >
          <Plus className="size-4" />
        </Button>
      </div>

      {/* Right Controls */}
      <div className="flex items-center gap-2 shrink-0">
        {/* Workspace Mode Switcher (Active only in Workspace) */}
        {surface === "workspace" && onModeChange && (
          <div className="hidden xl:flex items-center bg-slate-100 p-0.5 rounded-lg border border-slate-200 text-xs">
            <button
              type="button"
              onClick={() => onModeChange("SELECT")}
              className={`flex items-center gap-1 px-2.5 py-1 rounded-md font-medium transition-colors ${
                mode === "SELECT" ? "bg-white text-slate-900 shadow-2xs" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              <MousePointer className="size-3" />
              <span>Select</span>
            </button>
            <button
              type="button"
              onClick={() => onModeChange("TEXT")}
              className={`flex items-center gap-1 px-2.5 py-1 rounded-md font-medium transition-colors ${
                mode === "TEXT" ? "bg-white text-slate-900 shadow-2xs" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              <Type className="size-3" />
              <span>Edit Text</span>
            </button>
            <button
              type="button"
              onClick={() => onModeChange("FILL_AND_SIGN")}
              className={`flex items-center gap-1 px-2.5 py-1 rounded-md font-medium transition-colors ${
                mode === "FILL_AND_SIGN" ? "bg-white text-slate-900 shadow-2xs" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              <FileSpreadsheet className="size-3" />
              <span>Fill & Sign</span>
            </button>
            <button
              type="button"
              onClick={() => onModeChange("ANNOTATE")}
              className={`flex items-center gap-1 px-2.5 py-1 rounded-md font-medium transition-colors ${
                mode === "ANNOTATE" ? "bg-white text-slate-900 shadow-2xs" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              <MessageSquare className="size-3" />
              <span>Annotate</span>
            </button>
            <button
              type="button"
              onClick={() => onModeChange("PAGES")}
              className={`flex items-center gap-1 px-2.5 py-1 rounded-md font-medium transition-colors ${
                mode === "PAGES" ? "bg-white text-slate-900 shadow-2xs" : "text-slate-600 hover:text-slate-900"
              }`}
            >
              <Layers className="size-3" />
              <span>Pages</span>
            </button>
          </div>
        )}

        {/* Workspace Quick Actions */}
        {surface === "workspace" && (
          <div className="flex items-center gap-1">
            {onOpenCommandPalette && (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={onOpenCommandPalette}
                className="h-8 px-2.5 text-xs text-slate-600 gap-1 rounded-lg hidden md:inline-flex"
                title="Command Palette (Cmd+K)"
              >
                <Command className="size-3" />
                <span>Commands</span>
              </Button>
            )}

            {onExport && (
              <Button
                type="button"
                size="sm"
                onClick={onExport}
                className="h-8 px-3 rounded-lg bg-sky-600 hover:bg-sky-700 text-white text-xs font-medium gap-1.5 shadow-2xs"
                data-testid="navbar-export-btn"
              >
                <Download className="size-3.5" />
                <span>Export</span>
              </Button>
            )}
          </div>
        )}

        {/* Privacy Pill Reassurance */}
        <div className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200/70">
          <ShieldCheck className="size-3.5 text-emerald-600" />
          <span>Local-First</span>
        </div>
      </div>
    </header>
  );
}
