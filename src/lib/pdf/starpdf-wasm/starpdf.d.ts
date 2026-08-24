/* tslint:disable */
/* eslint-disable */

export function starpdf_add_annotation(handle: number, page_index: number, annotation_val: any): boolean;

export function starpdf_add_image(handle: number, page_index: number, image_bytes: Uint8Array, x: number, y: number, width: number, height: number): any;

export function starpdf_add_line(handle: number, input_val: any): any;

export function starpdf_add_rectangle(handle: number, input_val: any): any;

export function starpdf_close(handle: number): boolean;

export function starpdf_create_minimal_pdf(text: string): Uint8Array;

export function starpdf_delete_graphic(handle: number, input_val: any): any;

export function starpdf_delete_page(handle: number, page_index: number): Uint8Array;

export function starpdf_duplicate_page(handle: number, page_index: number, destination_index: number): Uint8Array;

export function starpdf_enumerate_all_graphics(handle: number): any;

export function starpdf_enumerate_graphics(handle: number, page_index: number): any;

export function starpdf_enumerate_images(handle: number, page_index: number): any;

export function starpdf_export_incremental(handle: number): Uint8Array;

export function starpdf_extract_all_text(handle: number): any;

export function starpdf_extract_page_text(handle: number, page_index: number): any;

export function starpdf_extract_pages(handle: number, page_indices: any): Uint8Array;

export function starpdf_get_annotations(handle: number, page_index: number): any;

export function starpdf_get_appearance_status(handle: number): string;

export function starpdf_get_form_fields(handle: number): any;

export function starpdf_get_glyph_mapping_quality(handle: number): string;

export function starpdf_get_info(handle: number): any;

export function starpdf_get_page_count(handle: number): number;

export function starpdf_get_security_info(handle: number): any;

export function starpdf_get_text_editability(handle: number, page_index: number, span_id: string): any;

export function starpdf_insert_blank_page(handle: number, page_index: number, width: number, height: number, rotation: number): Uint8Array;

export function starpdf_insert_imported_page(handle: number, imported_bytes: Uint8Array, imported_page_index: number, insert_at: number): Uint8Array;

export function starpdf_merge_documents(documents: any): Uint8Array;

export function starpdf_merge_selected(documents: any, page_sources: any): Uint8Array;

export function starpdf_move_page(handle: number, from_index: number, to_index: number): Uint8Array;

export function starpdf_move_text(handle: number, page_index: number, span_id: string, dx: number, dy: number): any;

export function starpdf_move_text_group(handle: number, page_index: number, span_ids: any, dx: number, dy: number): any;

export function starpdf_open(bytes: Uint8Array): number;

export function starpdf_plan_text_replacement(handle: number, page_index: number, span_id: string, new_text: string): any;

export function starpdf_remove_annotation(handle: number, page_index: number, obj_num: bigint, obj_gen: number): boolean;

export function starpdf_remove_image(handle: number, page_index: number, image_id: string): any;

export function starpdf_replace_image(handle: number, page_index: number, image_id: string, new_image_bytes: Uint8Array, clone_if_shared: boolean): any;

export function starpdf_replace_text(handle: number, page_index: number, span_id: string, new_text: string): any;

export function starpdf_replace_text_group(handle: number, page_index: number, span_ids: any, new_text: string): any;

export function starpdf_search(handle: number, query: string, case_sensitive: boolean): any;

export function starpdf_set_checkbox(handle: number, obj_num: bigint, obj_gen: number, checked: boolean): boolean;

export function starpdf_set_choice(handle: number, obj_num: bigint, obj_gen: number, value: string): boolean;

export function starpdf_set_choice_values(handle: number, obj_num: bigint, obj_gen: number, values_val: any): boolean;

export function starpdf_set_radio(handle: number, parent_num: bigint, parent_gen: number, widget_num: bigint, widget_gen: number, on_state: string): boolean;

export function starpdf_set_text_field(handle: number, obj_num: bigint, obj_gen: number, value: string): boolean;

export function starpdf_split_document(handle: number, ranges: any): any;

export function starpdf_update_annotation(handle: number, obj_num: bigint, obj_gen: number, update_val: any): boolean;

export function starpdf_update_graphic(handle: number, input_val: any): any;

export function starpdf_update_image(handle: number, page_index: number, image_id: string, new_x: number, new_y: number, new_width: number, new_height: number, clone_if_shared: boolean): any;

export function starpdf_validate(handle: number): boolean;

export function starpdf_version(): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly starpdf_add_annotation: (a: number, b: number, c: any) => [number, number, number];
    readonly starpdf_add_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number) => [number, number, number];
    readonly starpdf_add_line: (a: number, b: any) => [number, number, number];
    readonly starpdf_add_rectangle: (a: number, b: any) => [number, number, number];
    readonly starpdf_close: (a: number) => [number, number, number];
    readonly starpdf_create_minimal_pdf: (a: number, b: number) => [number, number, number, number];
    readonly starpdf_delete_graphic: (a: number, b: any) => [number, number, number];
    readonly starpdf_delete_page: (a: number, b: number) => [number, number, number, number];
    readonly starpdf_duplicate_page: (a: number, b: number, c: number) => [number, number, number, number];
    readonly starpdf_enumerate_all_graphics: (a: number) => [number, number, number];
    readonly starpdf_enumerate_graphics: (a: number, b: number) => [number, number, number];
    readonly starpdf_enumerate_images: (a: number, b: number) => [number, number, number];
    readonly starpdf_export_incremental: (a: number) => [number, number, number, number];
    readonly starpdf_extract_all_text: (a: number) => [number, number, number];
    readonly starpdf_extract_page_text: (a: number, b: number) => [number, number, number];
    readonly starpdf_extract_pages: (a: number, b: any) => [number, number, number, number];
    readonly starpdf_get_annotations: (a: number, b: number) => [number, number, number];
    readonly starpdf_get_appearance_status: (a: number) => [number, number, number, number];
    readonly starpdf_get_form_fields: (a: number) => [number, number, number];
    readonly starpdf_get_glyph_mapping_quality: (a: number) => [number, number, number, number];
    readonly starpdf_get_info: (a: number) => [number, number, number];
    readonly starpdf_get_page_count: (a: number) => [number, number, number];
    readonly starpdf_get_security_info: (a: number) => [number, number, number];
    readonly starpdf_get_text_editability: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly starpdf_insert_blank_page: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly starpdf_insert_imported_page: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly starpdf_merge_documents: (a: any) => [number, number, number, number];
    readonly starpdf_merge_selected: (a: any, b: any) => [number, number, number, number];
    readonly starpdf_move_page: (a: number, b: number, c: number) => [number, number, number, number];
    readonly starpdf_move_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly starpdf_move_text_group: (a: number, b: number, c: any, d: number, e: number) => [number, number, number];
    readonly starpdf_open: (a: number, b: number) => [number, number, number];
    readonly starpdf_plan_text_replacement: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly starpdf_remove_annotation: (a: number, b: number, c: bigint, d: number) => [number, number, number];
    readonly starpdf_remove_image: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly starpdf_replace_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly starpdf_replace_text: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly starpdf_replace_text_group: (a: number, b: number, c: any, d: number, e: number) => [number, number, number];
    readonly starpdf_search: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly starpdf_set_checkbox: (a: number, b: bigint, c: number, d: number) => [number, number, number];
    readonly starpdf_set_choice: (a: number, b: bigint, c: number, d: number, e: number) => [number, number, number];
    readonly starpdf_set_choice_values: (a: number, b: bigint, c: number, d: any) => [number, number, number];
    readonly starpdf_set_radio: (a: number, b: bigint, c: number, d: bigint, e: number, f: number, g: number) => [number, number, number];
    readonly starpdf_set_text_field: (a: number, b: bigint, c: number, d: number, e: number) => [number, number, number];
    readonly starpdf_split_document: (a: number, b: any) => [number, number, number];
    readonly starpdf_update_annotation: (a: number, b: bigint, c: number, d: any) => [number, number, number];
    readonly starpdf_update_graphic: (a: number, b: any) => [number, number, number];
    readonly starpdf_update_image: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => [number, number, number];
    readonly starpdf_validate: (a: number) => [number, number, number];
    readonly starpdf_version: () => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
