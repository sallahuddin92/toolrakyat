import type {
  StarPdfAddAnnotationInput,
  StarPdfAnnotation,
  StarPdfDocumentInfo,
  StarPdfFormField,
  StarPdfPageText,
  StarPdfSearchOptions,
  StarPdfSearchResult,
  StarPdfUpdateAnnotationInput,
} from "./starpdf-types";

// Import wasm module functions directly for universal execution (Node / SSR / Vitest / Main fallback)
import initWasm, {
  starpdf_add_annotation,
  starpdf_close,
  starpdf_create_minimal_pdf,
  starpdf_export_incremental,
  starpdf_extract_all_text,
  starpdf_extract_page_text,
  starpdf_get_annotations,
  starpdf_get_appearance_status,
  starpdf_get_glyph_mapping_quality,
  starpdf_get_form_fields,
  starpdf_get_info,
  starpdf_get_page_count,
  starpdf_open,
  starpdf_remove_annotation,
  starpdf_search,
  starpdf_set_checkbox,
  starpdf_set_choice,
  starpdf_set_choice_values,
  starpdf_set_radio,
  starpdf_set_text_field,
  starpdf_update_annotation,
  starpdf_validate,
  starpdf_version,
} from "./starpdf-wasm/starpdf";

let wasmInitialized: Promise<void> | null = null;

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

  constructor(handle: number) {
    this._handle = handle;
  }

  get handle(): number {
    return this._handle;
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

  async exportIncremental(): Promise<Uint8Array> {
    this.assertOpen();
    await ensureWasmInitialized();
    return starpdf_export_incremental(this._handle);
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
    const handle = starpdf_open(bytes);
    return new StarPdfDocumentHandle(handle);
  }

  static async createMinimalPdf(text: string): Promise<Uint8Array> {
    await ensureWasmInitialized();
    return starpdf_create_minimal_pdf(text);
  }
}
