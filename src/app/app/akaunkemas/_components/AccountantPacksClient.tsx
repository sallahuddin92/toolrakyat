"use client";

import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Label } from "@/components/ui/label";
import { Card, CardContent } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Package, Plus, X, Download, Trash2, FileArchive } from "lucide-react";
import { generatePack, deletePack, updatePackStatus } from "../accountant-packs/actions";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface PackRow {
  id: string;
  label: string;
  periodStart: string;
  periodEnd: string;
  status: "draft" | "generated" | "sent" | "archived";
  notes: string;
  generatedAt: string | null;
  createdAt: string;
}

interface Props {
  initialPacks: PackRow[];
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatDate(d: string): string {
  return new Date(d).toLocaleDateString("en-MY", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

const STATUS_STYLES: Record<string, string> = {
  draft: "bg-yellow-100 text-yellow-800",
  generated: "bg-green-100 text-green-800",
  sent: "bg-blue-100 text-blue-800",
  archived: "bg-slate-200 text-slate-600",
};

// ---------------------------------------------------------------------------
// Generate Pack Modal
// ---------------------------------------------------------------------------

function GenerateModal({
  open,
  onClose,
}: {
  open: boolean;
  onClose: () => void;
}) {
  const [generating, setGenerating] = useState(false);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setGenerating(true);
    const formData = new FormData(e.currentTarget);
    await generatePack(formData);
    setGenerating(false);
    onClose();
    window.location.reload();
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40">
      <div className="w-full max-w-md rounded-2xl bg-white p-6 shadow-xl">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-lg font-semibold text-slate-900">Generate New Pack</h2>
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="size-4" />
          </Button>
        </div>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label className="text-xs">Label</Label>
            <Input
              name="label"
              placeholder="e.g. January 2026 Pack"
              required
              className="h-9 text-sm"
            />
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label className="text-xs">Period Start</Label>
              <Input
                name="periodStart"
                type="date"
                required
                className="h-9 text-sm"
              />
            </div>
            <div className="space-y-1.5">
              <Label className="text-xs">Period End</Label>
              <Input
                name="periodEnd"
                type="date"
                required
                className="h-9 text-sm"
              />
            </div>
          </div>

          <div className="space-y-1.5">
            <Label className="text-xs">Notes (optional)</Label>
            <Input
              name="notes"
              placeholder="Any notes for the accountant..."
              className="h-9 text-sm"
            />
          </div>

          <div className="flex justify-end gap-3 pt-2">
            <Button type="button" variant="outline" onClick={onClose}>
              Cancel
            </Button>
            <Button type="submit" disabled={generating} className="gap-2">
              <FileArchive className="size-4" />
              {generating ? "Generating..." : "Generate Pack"}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main Component
// ---------------------------------------------------------------------------

export function AccountantPacksClient({ initialPacks }: Props) {
  const [packs] = useState(initialPacks);
  const [modalOpen, setModalOpen] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this pack?")) return;
    setDeletingId(id);
    const formData = new FormData();
    formData.set("id", id);
    await deletePack(formData);
    setDeletingId(null);
    window.location.reload();
  };

  const handleStatusChange = async (id: string, status: string) => {
    const formData = new FormData();
    formData.set("id", id);
    formData.set("status", status);
    await updatePackStatus(formData);
    window.location.reload();
  };

  const handleDownload = async (pack: PackRow) => {
    // Re-generate and download
    const formData = new FormData();
    formData.set("label", pack.label);
    formData.set("periodStart", pack.periodStart);
    formData.set("periodEnd", pack.periodEnd);
    formData.set("notes", pack.notes);

    const result = await generatePack(formData);
    if (result.success) {
      window.location.reload();
    }
  };

  if (packs.length === 0) {
    return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight text-slate-900">Accountant Packs</h1>
            <p className="text-sm text-slate-500">
              Generate and download accountant-ready packages.
            </p>
          </div>
          <Button
            className="gap-2"
            onClick={() => setModalOpen(true)}
          >
            <Plus className="size-4" />
            Generate New Pack
          </Button>
        </div>

        <Card className="rounded-2xl">
          <CardContent className="flex flex-col items-center gap-4 py-12">
            <div className="flex size-16 items-center justify-center rounded-2xl bg-slate-100">
              <Package className="size-8 text-slate-400" />
            </div>
            <div className="text-center">
              <p className="text-sm font-medium text-slate-900">No packs generated yet</p>
              <p className="text-xs text-slate-500">
                Generate an accountant pack from matched transactions.
              </p>
            </div>
            <Button
              className="gap-2"
              onClick={() => setModalOpen(true)}
            >
              <Plus className="size-4" />
              Generate New Pack
            </Button>
          </CardContent>
        </Card>

        <GenerateModal open={modalOpen} onClose={() => setModalOpen(false)} />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">Accountant Packs</h1>
          <p className="text-sm text-slate-500">
            {packs.length} pack{packs.length !== 1 ? "s" : ""} generated
          </p>
        </div>
        <Button
          className="gap-2"
          onClick={() => setModalOpen(true)}
        >
          <Plus className="size-4" />
          Generate New Pack
        </Button>
      </div>

      {/* Packs list */}
      <Card className="rounded-2xl overflow-hidden">
        <div className="grid grid-cols-12 gap-2 border-b border-slate-200 bg-slate-50 px-4 py-2.5 text-xs font-semibold uppercase tracking-wider text-slate-500">
          <div className="col-span-2">Label</div>
          <div className="col-span-2">Period</div>
          <div className="col-span-2">Status</div>
          <div className="col-span-2">Generated</div>
          <div className="col-span-2">Notes</div>
          <div className="col-span-2 text-right">Actions</div>
        </div>

        <div className="divide-y divide-slate-100">
          {packs.map((pack) => (
            <div
              key={pack.id}
              className="grid grid-cols-12 gap-2 px-4 py-2.5 text-sm hover:bg-slate-50"
            >
              <div className="col-span-2 self-center font-medium text-slate-900 truncate">
                {pack.label}
              </div>
              <div className="col-span-2 self-center text-xs text-slate-500">
                {formatDate(pack.periodStart)} — {formatDate(pack.periodEnd)}
              </div>
              <div className="col-span-2 self-center">
                <Badge
                  variant="secondary"
                  className={cn("text-[10px] h-5 px-1.5", STATUS_STYLES[pack.status])}
                >
                  {pack.status}
                </Badge>
              </div>
              <div className="col-span-2 self-center text-xs text-slate-500">
                {pack.generatedAt ? formatDate(pack.generatedAt) : "—"}
              </div>
              <div className="col-span-2 self-center text-xs text-slate-400 truncate">
                {pack.notes || "—"}
              </div>
              <div className="col-span-2 self-center flex items-center justify-end gap-1">
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-7"
                  onClick={() => handleDownload(pack)}
                  title="Download"
                >
                  <Download className="size-3 text-sky-500" />
                </Button>
                <Select
                  value={pack.status}
                  onValueChange={(v) => handleStatusChange(pack.id, v)}
                >
                  <SelectTrigger className="h-7 text-[10px] w-[90px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="draft">Draft</SelectItem>
                    <SelectItem value="generated">Generated</SelectItem>
                    <SelectItem value="sent">Sent</SelectItem>
                    <SelectItem value="archived">Archived</SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  size="icon"
                  variant="ghost"
                  className="size-7"
                  onClick={() => handleDelete(pack.id)}
                  disabled={deletingId === pack.id}
                  title="Delete"
                >
                  <Trash2 className="size-3 text-red-400" />
                </Button>
              </div>
            </div>
          ))}
        </div>
      </Card>

      <GenerateModal open={modalOpen} onClose={() => setModalOpen(false)} />
    </div>
  );
}
