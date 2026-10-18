"use client";

import React, { useState, useMemo, useRef } from "react";
import {
  Search,
  Type,
  Image as ImageIcon,
  Square,
  LayoutGrid,
  Files,
  Scissors,
  FilePlus2,
  Copy,
  FileSpreadsheet,
  Sparkles,
  PenTool,
  CheckSquare,
  MessageSquare,
  Highlighter,
  PenLine,
  Info,
  ShieldCheck,
  Download,
  ArrowRight,
  Filter,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  SMARTPDF_TOOLS,
  PDF_WORKFLOW_CATEGORIES,
} from "./pdfToolsCatalog";
import type { PdfToolDefinition, PdfWorkflowCategory } from "../types";
import type { EditorMode } from "../workspace/PdfToolbar";

interface SmartPdfAllToolsProps {
  onOpenFileWithTool: (file: File, tool: PdfToolDefinition) => void;
  onActivateToolForCurrentDoc?: (tool: PdfToolDefinition) => void;
  hasActiveDocument?: boolean;
}

export function SmartPdfAllTools({
  onOpenFileWithTool,
  onActivateToolForCurrentDoc,
  hasActiveDocument = false,
}: SmartPdfAllToolsProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [selectedCategory, setSelectedCategory] = useState<
    PdfWorkflowCategory | "all"
  >("all");
  const fileInputRef = useRef<HTMLInputElement>(null);
  const pendingToolRef = useRef<PdfToolDefinition | null>(null);

  const filteredTools = useMemo(() => {
    return SMARTPDF_TOOLS.filter((tool) => {
      const matchesCategory =
        selectedCategory === "all" || tool.category === selectedCategory;
      const query = searchQuery.trim().toLowerCase();
      const matchesSearch =
        !query ||
        tool.title.toLowerCase().includes(query) ||
        tool.description.toLowerCase().includes(query) ||
        tool.categoryLabel.toLowerCase().includes(query);
      return matchesCategory && matchesSearch;
    });
  }, [searchQuery, selectedCategory]);

  const handleToolClick = (tool: PdfToolDefinition) => {
    if (hasActiveDocument && onActivateToolForCurrentDoc) {
      onActivateToolForCurrentDoc(tool);
    } else {
      pendingToolRef.current = tool;
      fileInputRef.current?.click();
    }
  };

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file && pendingToolRef.current) {
      onOpenFileWithTool(file, pendingToolRef.current);
    }
    e.target.value = "";
    pendingToolRef.current = null;
  };

  const getToolIcon = (name: string) => {
    switch (name) {
      case "Type":
        return <Type className="size-5 text-sky-600" />;
      case "Image":
        return <ImageIcon className="size-5 text-sky-600" />;
      case "Square":
        return <Square className="size-5 text-sky-600" />;
      case "LayoutGrid":
        return <LayoutGrid className="size-5 text-indigo-600" />;
      case "Files":
        return <Files className="size-5 text-indigo-600" />;
      case "Scissors":
        return <Scissors className="size-5 text-indigo-600" />;
      case "FilePlus2":
        return <FilePlus2 className="size-5 text-indigo-600" />;
      case "Copy":
        return <Copy className="size-5 text-indigo-600" />;
      case "FileSpreadsheet":
        return <FileSpreadsheet className="size-5 text-emerald-600" />;
      case "Sparkles":
        return <Sparkles className="size-5 text-emerald-600" />;
      case "PenTool":
        return <PenTool className="size-5 text-emerald-600" />;
      case "CheckSquare":
        return <CheckSquare className="size-5 text-emerald-600" />;
      case "MessageSquare":
        return <MessageSquare className="size-5 text-amber-600" />;
      case "Highlighter":
        return <Highlighter className="size-5 text-amber-600" />;
      case "PenLine":
        return <PenLine className="size-5 text-amber-600" />;
      case "Info":
        return <Info className="size-5 text-slate-600" />;
      case "ShieldCheck":
        return <ShieldCheck className="size-5 text-slate-600" />;
      case "Download":
        return <Download className="size-5 text-slate-600" />;
      default:
        return <Type className="size-5 text-slate-600" />;
    }
  };

  return (
    <div
      className="flex-1 overflow-y-auto bg-slate-50/70 p-4 sm:p-8 select-none"
      data-testid="smartpdf-all-tools-view"
    >
      <input
        ref={fileInputRef}
        type="file"
        accept="application/pdf"
        className="hidden"
        onChange={handleFileChange}
      />

      <div className="max-w-6xl mx-auto space-y-6">
        {/* Header with Title and Search */}
        <div className="flex flex-col md:flex-row md:items-center justify-between gap-4">
          <div>
            <h1 className="text-2xl sm:text-3xl font-extrabold text-slate-900 tracking-tight">
              All PDF Tools
            </h1>
            <p className="text-sm text-slate-500 mt-1">
              Explore the complete professional PDF toolkit powered by StarPDF WebAssembly
            </p>
          </div>

          {/* Search Bar */}
          <div className="relative w-full md:w-80">
            <Search className="absolute left-3.5 top-1/2 -translate-y-1/2 size-4 text-slate-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search PDF tools…"
              className="w-full h-11 pl-10 pr-4 rounded-xl border border-slate-200 bg-white text-sm text-slate-900 placeholder-slate-400 shadow-2xs outline-none focus:border-sky-500 focus:ring-2 focus:ring-sky-100 transition-all"
              data-testid="tools-search-input"
            />
          </div>
        </div>

        {/* Workflow Category Filter Chips */}
        <div className="flex items-center gap-1.5 overflow-x-auto pb-1 text-xs">
          <Button
            type="button"
            size="sm"
            variant={selectedCategory === "all" ? "default" : "outline"}
            onClick={() => setSelectedCategory("all")}
            className={`h-8 rounded-full px-4 text-xs font-medium ${
              selectedCategory === "all"
                ? "bg-slate-900 text-white"
                : "border-slate-200 text-slate-600 hover:bg-slate-100"
            }`}
          >
            All Tools ({SMARTPDF_TOOLS.length})
          </Button>

          {PDF_WORKFLOW_CATEGORIES.map((cat) => {
            const count = SMARTPDF_TOOLS.filter((t) => t.category === cat.id).length;
            const isSelected = selectedCategory === cat.id;
            return (
              <Button
                key={cat.id}
                type="button"
                size="sm"
                variant={isSelected ? "default" : "outline"}
                onClick={() => setSelectedCategory(cat.id)}
                className={`h-8 rounded-full px-4 text-xs font-medium whitespace-nowrap ${
                  isSelected
                    ? "bg-slate-900 text-white"
                    : "border-slate-200 text-slate-600 hover:bg-slate-100"
                }`}
              >
                {cat.label} ({count})
              </Button>
            );
          })}
        </div>

        {/* Tools Grid */}
        {filteredTools.length === 0 ? (
          <div className="rounded-2xl border border-slate-200 bg-white p-12 text-center text-slate-400">
            <Filter className="size-8 mx-auto text-slate-300 mb-2" />
            <p className="text-sm font-medium text-slate-600">No matching tools found</p>
            <p className="text-xs text-slate-400 mt-1">
              Try searching with different keywords or switch categories above
            </p>
          </div>
        ) : (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4">
            {filteredTools.map((tool) => (
              <Card
                key={tool.id}
                onClick={() => handleToolClick(tool)}
                className="cursor-pointer rounded-2xl border border-slate-200/80 bg-white shadow-xs transition-all hover:shadow-md hover:border-sky-300 group"
                data-testid={`tool-card-${tool.id}`}
              >
                <CardContent className="p-5 flex flex-col justify-between h-full space-y-4">
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <div className="p-2.5 rounded-xl bg-slate-50 border border-slate-200/60 group-hover:bg-sky-50 transition-colors">
                        {getToolIcon(tool.iconName)}
                      </div>
                      <span className="text-[10px] font-semibold text-slate-400 uppercase tracking-wider bg-slate-50 px-2 py-0.5 rounded border border-slate-100">
                        {tool.categoryLabel}
                      </span>
                    </div>

                    <div>
                      <h3 className="font-semibold text-slate-900 text-sm group-hover:text-sky-600 transition-colors">
                        {tool.title}
                      </h3>
                      <p className="text-xs text-slate-500 mt-1 leading-relaxed">
                        {tool.description}
                      </p>
                    </div>
                  </div>

                  <div className="pt-2 flex items-center justify-between text-xs border-t border-slate-100 text-slate-600">
                    <span className="text-[11px] text-slate-400">
                      {hasActiveDocument ? "Apply to current PDF" : "Open PDF to use"}
                    </span>
                    <span className="font-medium text-sky-600 group-hover:translate-x-0.5 transition-transform flex items-center gap-1">
                      <span>Use tool</span>
                      <ArrowRight className="size-3" />
                    </span>
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
