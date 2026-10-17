"use client";

import React, { useState, useCallback, useEffect } from "react";
import { SmartPdfEditor } from "@/components/tools/implementations/pdf/SmartPdfEditor";
import { SmartPdfHome } from "./home/SmartPdfHome";
import { SmartPdfAllTools } from "./tools/SmartPdfAllTools";
import { SmartPdfNavbar } from "./layout/SmartPdfNavbar";
import { addRecentDocument, getRecentDocuments } from "./recent/recentFilesStore";
import type { SmartPdfSurface, DocumentTabSession, PdfToolDefinition } from "./types";
import type { EditorMode } from "@/components/tools/implementations/pdf/PdfToolbar";

export function SmartPdfApp() {
  const [surface, setSurface] = useState<SmartPdfSurface>("home");
  const [tabs, setTabs] = useState<DocumentTabSession[]>([]);
  const [activeTabId, setActiveTabId] = useState<string | null>(null);
  const [targetMode, setTargetMode] = useState<EditorMode | undefined>(undefined);

  const activeTab = tabs.find((t) => t.id === activeTabId) || null;

  const handleOpenDocument = useCallback(
    async (file: File, mode?: EditorMode) => {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const newTabId = `tab-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
      const newTab: DocumentTabSession = {
        id: newTabId,
        title: file.name,
        sourceBytes: bytes,
        isModified: false,
        pageCount: 1,
        initialMode: mode,
      };

      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTabId);
      setTargetMode(mode);
      setSurface("workspace");

      void addRecentDocument({
        name: file.name,
        bytes,
      });
    },
    [],
  );

  const handleOpenBytes = useCallback(
    (name: string, bytes: Uint8Array, mode?: EditorMode) => {
      const newTabId = `tab-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`;
      const newTab: DocumentTabSession = {
        id: newTabId,
        title: name,
        sourceBytes: bytes,
        isModified: false,
        pageCount: 1,
        initialMode: mode,
      };

      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTabId);
      setTargetMode(mode);
      setSurface("workspace");

      void addRecentDocument({
        name,
        bytes,
      });
    },
    [],
  );

  const handleCloseTab = useCallback(
    (tabId: string, e: React.MouseEvent) => {
      e.stopPropagation();
      setTabs((prev) => {
        const nextTabs = prev.filter((t) => t.id !== tabId);
        if (activeTabId === tabId) {
          if (nextTabs.length > 0) {
            setActiveTabId(nextTabs[nextTabs.length - 1].id);
          } else {
            setActiveTabId(null);
            setSurface("home");
          }
        }
        return nextTabs;
      });
    },
    [activeTabId],
  );

  const handleSelectTab = useCallback((tabId: string) => {
    setActiveTabId(tabId);
    setSurface("workspace");
  }, []);

  const handleOpenFilePrompt = useCallback(() => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "application/pdf";
    input.onchange = (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (file) {
        void handleOpenDocument(file);
      }
    };
    input.click();
  }, [handleOpenDocument]);

  return (
    <div
      className="h-full w-full flex flex-col bg-slate-100 overflow-hidden relative select-none"
      data-testid="smartpdf-standalone-application"
    >
      {/* Global Navbar */}
      <SmartPdfNavbar
        surface={surface}
        onSelectSurface={setSurface}
        tabs={tabs}
        activeTabId={activeTabId}
        onSelectTab={handleSelectTab}
        onCloseTab={handleCloseTab}
        onOpenNewFile={handleOpenFilePrompt}
      />

      {/* Surface 1: Home View */}
      {surface === "home" && (
        <SmartPdfHome
          onOpenFile={handleOpenDocument}
          onOpenBytes={handleOpenBytes}
          onNavigateToTools={() => setSurface("tools")}
        />
      )}

      {/* Surface 2: All Tools View */}
      {surface === "tools" && (
        <SmartPdfAllTools
          hasActiveDocument={Boolean(activeTab)}
          onOpenFileWithTool={async (file, tool) => {
            await handleOpenDocument(file, tool.recommendedMode);
          }}
          onActivateToolForCurrentDoc={(tool) => {
            if (tool.recommendedMode) {
              setTargetMode(tool.recommendedMode);
            }
            setSurface("workspace");
          }}
        />
      )}

      {/* Surface 3: Document Workspace */}
      {surface === "workspace" && (
        <div className="flex-1 overflow-hidden relative">
          {activeTab ? (
            <SmartPdfEditor
              key={activeTab.id}
              initialFile={{
                filename: activeTab.title,
                bytes: activeTab.sourceBytes,
              }}
              initialMode={targetMode || activeTab.initialMode}
              onHomeClick={() => setSurface("home")}
              onAllToolsClick={() => setSurface("tools")}
            />
          ) : (
            <SmartPdfHome
              onOpenFile={handleOpenDocument}
              onOpenBytes={handleOpenBytes}
              onNavigateToTools={() => setSurface("tools")}
            />
          )}
        </div>
      )}
    </div>
  );
}
