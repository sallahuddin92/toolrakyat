export const TOOL_CATEGORIES = [
  { id: "pdf", label: "PDF" },
  { id: "image", label: "Image" },
  { id: "compression", label: "Compression" },
  { id: "converter", label: "Converter" },
  { id: "text", label: "Text" },
  { id: "business", label: "Business" },
  { id: "calculator", label: "Calculator" },
  { id: "developer", label: "Developer" },
  { id: "qr", label: "QR" },
  { id: "ai", label: "AI" },
  { id: "akaunkemas", label: "AkaunKemas" },
] as const;

export type ToolCategoryId = (typeof TOOL_CATEGORIES)[number]["id"];
export type ToolCategoryLabel = (typeof TOOL_CATEGORIES)[number]["label"];

export type ToolDefinition = {
  id: string;
  name: string;
  slug: string;
  categoryId: ToolCategoryId;
  category: ToolCategoryLabel;
  description: string;
  icon: string;
  tags: string[];
  isImplemented: boolean;
  isPopular: boolean;
  route: string;
  acceptedFileTypes?: string[];
  maxFileSizeMB?: number;
  privacyNote?: string;
  seoTitle: string;
  seoDescription: string;
};

