import type { LucideIcon } from "lucide-react";
import {
  Archive,
  ArrowLeftRight,
  Braces,
  Calculator,
  Eraser,
  FileText,
  Files,
  FileOutput,
  FolderDown,
  FlipHorizontal2,
  ImagePlus,
  Info,
  Layers,
  MessageCircle,
  PenTool,
  Repeat2,
  Stamp,
  Trash2,
  Hash,
  Link2,
  QrCode,
  Receipt,
  RotateCw,
  Scan,
  Scissors,
  Shrink,
  Sparkles,
  Type,
} from "lucide-react";

const ICONS: Record<string, LucideIcon> = {
  Archive,
  ArrowLeftRight,
  Braces,
  Calculator,
  Eraser,
  FileText,
  Files,
  FileOutput,
  FolderDown,
  FlipHorizontal2,
  ImagePlus,
  Info,
  Layers,
  MessageCircle,
  PenTool,
  Repeat2,
  Stamp,
  Trash2,
  Hash,
  Link2,
  QrCode,
  Receipt,
  RotateCw,
  Scan,
  Scissors,
  Shrink,
  Sparkles,
  Type,
};

export function ToolIcon({
  name,
  className,
}: {
  name: string;
  className?: string;
}) {
  const Icon = ICONS[name] ?? FileText;
  return <Icon className={className} aria-hidden="true" />;
}
