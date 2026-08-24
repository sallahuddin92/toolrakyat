import type {
  StarPdfAddAnnotationInput,
  StarPdfAddLineInput,
  StarPdfAddRectangleInput,
  StarPdfAnnotation,
  StarPdfDeleteVectorGraphicInput,
  StarPdfDocumentInfo,
  StarPdfFormField,
  StarPdfImageInfo,
  StarPdfImageMutationResult,
  StarPdfMoveTextResult,
  StarPdfPageText,
  StarPdfReplaceTextResult,
  StarPdfSearchOptions,
  StarPdfSearchResult,
  StarPdfSecurityInfo,
  StarPdfTextSpan,
  StarPdfUpdateAnnotationInput,
  StarPdfUpdateVectorGraphicInput,
  StarPdfVectorGraphicInfo,
  StarPdfVectorMutationResult,
  StarPdfXrefStatus,
} from "./starpdf-types";

// Import wasm module functions directly for universal execution (Node / SSR / Vitest / Main fallback)
import initWasm, {
  starpdf_add_annotation,
  starpdf_close,
  starpdf_create_minimal_pdf,
  starpdf_delete_page,
  starpdf_duplicate_page,
  starpdf_export_incremental,
  starpdf_extract_all_text,
  starpdf_extract_page_text,
  starpdf_extract_pages,
  starpdf_get_annotations,
  starpdf_get_appearance_status,
  starpdf_get_glyph_mapping_quality,
  starpdf_get_form_fields,
  starpdf_get_info,
  starpdf_get_page_count,
  starpdf_get_security_info,
  starpdf_open,
  starpdf_register_font_asset,
  starpdf_insert_blank_page,
  starpdf_insert_imported_page,
  starpdf_merge_documents,
  starpdf_merge_selected,
  starpdf_move_page,
  starpdf_remove_annotation,
  starpdf_replace_text,
  starpdf_replace_text_group,
  starpdf_move_text,
  starpdf_move_text_group,
  starpdf_get_text_editability,


  starpdf_enumerate_images,
  starpdf_replace_image,
  starpdf_add_image,
  starpdf_update_image,
  starpdf_remove_image,

  starpdf_enumerate_graphics,
  starpdf_enumerate_all_graphics,
  starpdf_update_graphic,
  starpdf_add_rectangle,
  starpdf_add_line,
  starpdf_delete_graphic,
  starpdf_search,
  starpdf_split_document,
  starpdf_set_checkbox,
  starpdf_set_choice,
  starpdf_set_choice_values,
  starpdf_set_radio,
  starpdf_set_text_field,
  starpdf_update_annotation,
  starpdf_validate,
  starpdf_version,
} from "./starpdf-wasm/starpdf.js";

let wasmInitialized: Promise<void> | null = null;
const registeredFallbackFonts = new Set<string>();

const FALLBACK_FONT_ASSETS = {
  arabic: ["noto-sans-arabic", "NotoSansArabic-Regular.ttf"],
  hebrew: ["noto-sans-hebrew", "NotoSansHebrew-Regular.ttf"],
  devanagari: ["noto-sans-devanagari", "NotoSansDevanagari-Regular.ttf"],
  japanese: ["noto-sans-cjk-jp", "NotoSansJP-Regular.ttf"],
  korean: ["noto-sans-cjk-kr", "NotoSansKR-Regular.ttf"],
  simplifiedChinese: ["noto-sans-cjk-sc", "NotoSansSC-Regular.ttf"],
  traditionalChinese: ["noto-sans-cjk-tc", "NotoSansTC-Regular.ttf"],
} as const;

type FallbackFontKind = keyof typeof FALLBACK_FONT_ASSETS;

function fallbackFontsForText(text: string): FallbackFontKind[] {
  const kinds = new Set<FallbackFontKind>();
  if (/\p{Script=Arabic}/u.test(text)) kinds.add("arabic");
  if (/\p{Script=Hebrew}/u.test(text)) kinds.add("hebrew");
  if (/\p{Script=Devanagari}/u.test(text)) kinds.add("devanagari");
  if (/\p{Script=Hangul}/u.test(text)) kinds.add("korean");
  if (/[\u3040-\u30ff\u31f0-\u31ff]/u.test(text)) kinds.add("japanese");
  if (/\p{Script=Han}/u.test(text) && !kinds.has("japanese") && !kinds.has("korean")) {
    // Locale cannot be inferred perfectly from Han alone. Prefer TC when the replacement
    // contains common Traditional-only forms; otherwise use the SC fallback.
    if (/[體測試繁國語學書車門風馬龍臺灣]/u.test(text)) {
      kinds.add("traditionalChinese");
    } else {
      kinds.add("simplifiedChinese");
    }
  }
  return [...kinds];
}

async function registerFallbackFontsForText(text: string): Promise<void> {
  await ensureWasmInitialized();
  await Promise.all(
    fallbackFontsForText(text).map(async (kind) => {
      const [fontId, filename] = FALLBACK_FONT_ASSETS[kind];
      if (registeredFallbackFonts.has(fontId)) return;

      let bytes: Uint8Array;
      if (typeof window === "undefined") {
        const fs = await import("fs");
        const path = await import("path");
        bytes = fs.readFileSync(path.resolve(process.cwd(), "public/fonts", filename));
      } else {
        const response = await fetch(`/fonts/${filename}`);
        if (!response.ok) {
          throw new Error(`Failed to load fallback font ${fontId}: HTTP ${response.status}`);
        }
        bytes = new Uint8Array(await response.arrayBuffer());
      }
      starpdf_register_font_asset(fontId, bytes);
      registeredFallbackFonts.add(fontId);
    }),
  );
}

export async function ensureWasmInitialized(): Promise<void> {
  if (!wasmInitialized) {
    wasmInitialized = (async () => {
      if (typeof window === "undefined") {
        // Node / Vitest environment: load bytes from filesystem
        const fs = await import("fs");
        const path = await import("path");
        const wasmPath = path.resolve(process.cwd(), "public/starpdf_wasm/starpdf_bg.wasm");
        if (fs.existsSync(wasmPath)) {
          const wasmBuffer = fs.readFileSync(wasmPath);
          await initWasm({ module_or_path: wasmBuffer });
        } else {
          // Fallback to local wasm directory
          const localPath = path.resolve(
            process.cwd(),
            "src/lib/pdf/starpdf-wasm/starpdf_bg.wasm"
          );
          const wasmBuffer = fs.readFileSync(localPath);
          await initWasm({ module_or_path: wasmBuffer });
        }
      } else {
        // Browser environment: load from public URL
        await initWasm({ module_or_path: "/starpdf_wasm/starpdf_bg.wasm" });
      }
    })();
  }
  return wasmInitialized;
}

export class StarPdfDocumentHandle {
  private _handle: number;
  private _isClosed = false;
  private _xrefStatus: StarPdfXrefStatus = "VALID";

  constructor(handle: number) {
    this._handle = handle;
  }

  get handle(): number {
    return this._handle;
  }

  get xrefStatus(): StarPdfXrefStatus {
    return this._xrefStatus;
  }

  setXrefStatus(status: StarPdfXrefStatus): void {
    this._xrefStatus = status;
  }

  get isClosed(): boolean {
    return this._isClosed;
  }

  private assertOpen(): void {
    if (this._isClosed) {
      throw new Error(`Document handle ${this._handle} has been closed.`);
    }
  }

  async getInfo(): Promise<StarPdfDocumentInfo> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_info(this._handle) as StarPdfDocumentInfo;
  }

  async getPageCount(): Promise<number> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_page_count(this._handle);
  }

  async getSecurityInfo(): Promise<StarPdfSecurityInfo> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_security_info(this._handle) as StarPdfSecurityInfo;
  }

  async extractPageText(pageIndex: number): Promise<StarPdfPageText> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_extract_page_text(this._handle, pageIndex) as StarPdfPageText;
  }

  async extractAllText(): Promise<StarPdfPageText[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_extract_all_text(this._handle) as StarPdfPageText[];
  }

  async search(
    query: string,
    options: StarPdfSearchOptions = {}
  ): Promise<StarPdfSearchResult[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    const caseSensitive = Boolean(options.caseSensitive);
    return starpdf_search(
      this._handle,
      query,
      caseSensitive
    ) as StarPdfSearchResult[];
  }

  async validate(): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_validate(this._handle);
  }

  async getFormFields(): Promise<StarPdfFormField[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_form_fields(this._handle) as StarPdfFormField[];
  }

  async getAnnotations(pageIndex: number): Promise<StarPdfAnnotation[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_annotations(this._handle, pageIndex) as StarPdfAnnotation[];
  }

  async setTextField(
    objectNum: number,
    objectGen: number,
    value: string
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_set_text_field(
      this._handle,
      BigInt(objectNum),
      objectGen,
      value
    );
  }

  async setCheckbox(
    objectNum: number,
    objectGen: number,
    checked: boolean
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_set_checkbox(
      this._handle,
      BigInt(objectNum),
      objectGen,
      checked
    );
  }

  async setRadio(
    parentNum: number,
    parentGen: number,
    widgetNum: number,
    widgetGen: number,
    onState: string
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_set_radio(
      this._handle,
      BigInt(parentNum),
      parentGen,
      BigInt(widgetNum),
      widgetGen,
      onState
    );
  }

  async setChoice(
    objectNum: number,
    objectGen: number,
    value: string
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_set_choice(
      this._handle,
      BigInt(objectNum),
      objectGen,
      value
    );
  }

  async setChoiceValues(
    objectNum: number,
    objectGen: number,
    values: string[],
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_set_choice_values(
      this._handle,
      BigInt(objectNum),
      objectGen,
      values,
    );
  }

  async addAnnotation(
    pageIndex: number,
    input: StarPdfAddAnnotationInput
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_add_annotation(this._handle, pageIndex, input);
  }

  async updateAnnotation(
    objectNum: number,
    objectGen: number,
    input: StarPdfUpdateAnnotationInput
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_update_annotation(
      this._handle,
      BigInt(objectNum),
      objectGen,
      input
    );
  }

  async removeAnnotation(
    pageIndex: number,
    objectNum: number,
    objectGen: number
  ): Promise<boolean> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_remove_annotation(
      this._handle,
      pageIndex,
      BigInt(objectNum),
      objectGen
    );
  }

  async replaceText(
    pageIndex: number,
    spanId: string,
    newText: string
  ): Promise<StarPdfReplaceTextResult> {
    this.assertOpen();
    await registerFallbackFontsForText(newText);
    return starpdf_replace_text(
      this._handle,
      pageIndex,
      spanId,
      newText
    ) as StarPdfReplaceTextResult;
  }

  async replaceTextGroup(
    pageIndex: number,
    spanIds: string[],
    newText: string
  ): Promise<StarPdfReplaceTextResult> {
    this.assertOpen();
    await registerFallbackFontsForText(newText);
    return starpdf_replace_text_group(
      this._handle,
      pageIndex,
      spanIds,
      newText
    ) as StarPdfReplaceTextResult;
  }

  async moveText(
    pageIndex: number,
    spanId: string,
    dx: number,
    dy: number
  ): Promise<StarPdfMoveTextResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_move_text(
      this._handle,
      pageIndex,
      spanId,
      dx,
      dy
    ) as StarPdfMoveTextResult;
  }

  async moveTextGroup(
    pageIndex: number,
    spanIds: string[],
    dx: number,
    dy: number
  ): Promise<StarPdfMoveTextResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_move_text_group(
      this._handle,
      pageIndex,
      spanIds,
      dx,
      dy
    ) as StarPdfMoveTextResult;
  }



  async getTextEditability(
    pageIndex: number,
    spanId: string
  ): Promise<StarPdfTextSpan> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_text_editability(
      this._handle,
      pageIndex,
      spanId
    ) as StarPdfTextSpan;
  }

  async enumerateImages(pageIndex: number): Promise<StarPdfImageInfo[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_enumerate_images(
      this._handle,
      pageIndex
    ) as StarPdfImageInfo[];
  }

  async replaceImage(
    pageIndex: number,
    imageId: string,
    newImageBytes: Uint8Array,
    cloneIfShared = true
  ): Promise<StarPdfImageMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_replace_image(
      this._handle,
      pageIndex,
      imageId,
      newImageBytes,
      cloneIfShared
    ) as StarPdfImageMutationResult;
  }

  async addImage(
    pageIndex: number,
    imageBytes: Uint8Array,
    x: number,
    y: number,
    width: number,
    height: number
  ): Promise<StarPdfImageMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_add_image(
      this._handle,
      pageIndex,
      imageBytes,
      x,
      y,
      width,
      height
    ) as StarPdfImageMutationResult;
  }

  async updateImage(
    pageIndex: number,
    imageId: string,
    x: number,
    y: number,
    width: number,
    height: number,
    cloneIfShared = true
  ): Promise<StarPdfImageMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_update_image(
      this._handle,
      pageIndex,
      imageId,
      x,
      y,
      width,
      height,
      cloneIfShared
    ) as StarPdfImageMutationResult;
  }

  async removeImage(
    pageIndex: number,
    imageId: string
  ): Promise<StarPdfImageMutationResult> {

    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_remove_image(
      this._handle,
      pageIndex,
      imageId
    ) as StarPdfImageMutationResult;
  }

  async enumerateGraphics(pageIndex: number): Promise<StarPdfVectorGraphicInfo[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_enumerate_graphics(
      this._handle,
      pageIndex
    ) as StarPdfVectorGraphicInfo[];
  }

  async enumerateAllGraphics(): Promise<StarPdfVectorGraphicInfo[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_enumerate_all_graphics(
      this._handle
    ) as StarPdfVectorGraphicInfo[];
  }

  async updateGraphic(
    input: StarPdfUpdateVectorGraphicInput
  ): Promise<StarPdfVectorMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_update_graphic(
      this._handle,
      input
    ) as StarPdfVectorMutationResult;
  }

  async addRectangle(
    input: StarPdfAddRectangleInput
  ): Promise<StarPdfVectorMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_add_rectangle(
      this._handle,
      input
    ) as StarPdfVectorMutationResult;
  }

  async addLine(
    input: StarPdfAddLineInput
  ): Promise<StarPdfVectorMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_add_line(
      this._handle,
      input
    ) as StarPdfVectorMutationResult;
  }

  async deleteGraphic(
    input: StarPdfDeleteVectorGraphicInput
  ): Promise<StarPdfVectorMutationResult> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_delete_graphic(
      this._handle,
      input
    ) as StarPdfVectorMutationResult;
  }

  async exportIncremental(): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_export_incremental(this._handle);
  }

  async deletePage(pageIndex: number): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_delete_page(this._handle, pageIndex);
  }

  async movePage(fromIndex: number, toIndex: number): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_move_page(this._handle, fromIndex, toIndex);
  }

  async duplicatePage(pageIndex: number, destinationIndex: number): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_duplicate_page(this._handle, pageIndex, destinationIndex);
  }

  async insertBlankPage(
    pageIndex: number,
    width: number,
    height: number,
    rotation: 0 | 90 | 180 | 270 = 0
  ): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_insert_blank_page(
      this._handle,
      pageIndex,
      width,
      height,
      rotation
    );
  }

  async extractPages(pageIndices: number[]): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_extract_pages(this._handle, pageIndices);
  }

  async insertImportedPage(
    importedBytes: Uint8Array,
    importedPageIndex: number,
    destinationIndex: number
  ): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_insert_imported_page(
      this._handle,
      importedBytes,
      importedPageIndex,
      destinationIndex
    );
  }

  async splitDocument(
    ranges: { start: number; endExclusive: number }[]
  ): Promise<Uint8Array[]> {
    this.assertOpen();
    await ensureWasmInitialized();
    const outputs = starpdf_split_document(
      this._handle,
      ranges.map((range) => [range.start, range.endExclusive])
    ) as number[][];
    return outputs.map((output) => new Uint8Array(output));
  }

  async getAppearanceStatus(): Promise<
    "AP_REGENERATED" | "AP_PRESERVED" | "AP_NOT_REQUIRED" | "AP_UNSUPPORTED"
  > {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_appearance_status(this._handle) as
      | "AP_REGENERATED"
      | "AP_PRESERVED"
      | "AP_NOT_REQUIRED"
      | "AP_UNSUPPORTED";
  }

  async getGlyphMappingQuality(): Promise<
    "EXACT" | "FALLBACK" | "UNREPRESENTABLE" | "NOT_APPLICABLE"
  > {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_get_glyph_mapping_quality(this._handle) as
      | "EXACT"
      | "FALLBACK"
      | "UNREPRESENTABLE"
      | "NOT_APPLICABLE";
  }

  async close(): Promise<void> {
    if (this._isClosed) return;
    await ensureWasmInitialized();
    starpdf_close(this._handle);
    this._isClosed = true;
  }
}

export class StarPdfClient {
  static async getVersion(): Promise<string> {
    await ensureWasmInitialized();
    return starpdf_version();
  }

  static async open(
    data: ArrayBuffer | Uint8Array
  ): Promise<StarPdfDocumentHandle> {
    await ensureWasmInitialized();
    const bytes = data instanceof Uint8Array ? data : new Uint8Array(data);
    let document: StarPdfDocumentHandle | null = null;
    try {
      const handle = starpdf_open(bytes);
      document = new StarPdfDocumentHandle(handle);
      const info = await document.getInfo();
      document.setXrefStatus(info.xref_status);
      return document;
    } catch (error) {
      await document?.close();
      throw new StarPdfOpenError(error);
    }
  }

  static async createMinimalPdf(text: string): Promise<Uint8Array> {
    await ensureWasmInitialized();
    return starpdf_create_minimal_pdf(text);
  }

  static async mergeDocuments(
    documents: Uint8Array[],
    pageSources?: { documentIndex: number; pageIndex: number }[]
  ): Promise<Uint8Array> {
    await ensureWasmInitialized();
    if (pageSources) {
      return starpdf_merge_selected(
        documents,
        pageSources.map((source) => [source.documentIndex, source.pageIndex])
      );
    }
    return starpdf_merge_documents(documents);
  }
}

export class StarPdfOpenError extends Error {
  readonly xref_status = "UNRECOVERABLE" as const;

  constructor(cause: unknown) {
    super("StarPDF could not establish a coherent document graph for native editing.", {
      cause,
    });
    this.name = "StarPdfOpenError";
  }
}
