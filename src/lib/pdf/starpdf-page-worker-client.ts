import type {
  StarPdfWorkerRequest,
  StarPdfWorkerResponse,
} from "./starpdf-types";
import { StarPdfClient } from "./starpdf-client";

export type StarPdfPageOperation =
  | { type: "deletePage"; pageIndex: number }
  | { type: "movePage"; fromIndex: number; toIndex: number }
  | { type: "duplicatePage"; pageIndex: number; destinationIndex: number }
  | {
      type: "insertBlankPage";
      pageIndex: number;
      width: number;
      height: number;
      rotation: 0 | 90 | 180 | 270;
    }
  | { type: "extractPages"; pageIndices: number[] };

function requestId(type: string): string {
  return `${type}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function sendRequest(
  worker: Worker,
  request: StarPdfWorkerRequest,
  transfer: Transferable[] = []
): Promise<StarPdfWorkerResponse> {
  return new Promise((resolve, reject) => {
    const onMessage = (event: MessageEvent<StarPdfWorkerResponse>) => {
      if (event.data.id !== request.id) return;
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onError);
      if (!event.data.success) {
        const error = new Error(event.data.error);
        error.name = event.data.code;
        reject(error);
        return;
      }
      resolve(event.data);
    };
    const onError = (event: ErrorEvent) => {
      worker.removeEventListener("message", onMessage);
      worker.removeEventListener("error", onError);
      reject(new Error(event.message || "StarPDF worker failed"));
    };
    worker.addEventListener("message", onMessage);
    worker.addEventListener("error", onError);
    worker.postMessage(request, transfer);
  });
}

export async function runStarPdfPageOperation(
  input: Uint8Array,
  operation: StarPdfPageOperation
): Promise<Uint8Array> {
  if (typeof Worker === "undefined") {
    const handle = await StarPdfClient.open(input);
    let output: Uint8Array;
    switch (operation.type) {
      case "deletePage":
        output = await handle.deletePage(operation.pageIndex);
        break;
      case "movePage":
        output = await handle.movePage(operation.fromIndex, operation.toIndex);
        break;
      case "duplicatePage":
        output = await handle.duplicatePage(operation.pageIndex, operation.destinationIndex);
        break;
      case "insertBlankPage":
        output = await handle.insertBlankPage(
          operation.pageIndex,
          operation.width,
          operation.height,
          operation.rotation
        );
        break;
      case "extractPages":
        output = await handle.extractPages(operation.pageIndices);
        break;
    }
    await handle.close();
    return output;
  }

  const worker = new Worker("/starpdf.worker.js", { type: "module" });
  let handle: number | undefined;
  try {
    await sendRequest(worker, { type: "init", id: requestId("init") });
    const source = input.slice();
    const buffer = source.buffer as ArrayBuffer;
    const opened = await sendRequest(
      worker,
      { type: "open", id: requestId("open"), buffer },
      [buffer]
    );
    if (opened.type !== "open") throw new Error("Unexpected StarPDF worker response");
    handle = opened.handle;

    const id = requestId(operation.type);
    const request: StarPdfWorkerRequest = { ...operation, id, handle };
    const response = await sendRequest(worker, request);
    if (!("bytes" in response)) throw new Error("StarPDF page operation returned no bytes");
    return response.bytes;
  } finally {
    if (handle !== undefined) {
      try {
        await sendRequest(worker, { type: "close", id: requestId("close"), handle });
      } catch {
        // Terminating the dedicated worker still releases its private registry.
      }
    }
    worker.terminate();
  }
}

export async function mergeStarPdfDocuments(
  inputs: Uint8Array[],
  pageSources?: { documentIndex: number; pageIndex: number }[]
): Promise<Uint8Array> {
  const safeInputs = inputs.length === 1 ? [inputs[0], inputs[0]] : inputs;

  if (typeof Worker === "undefined") {
    return StarPdfClient.mergeDocuments(safeInputs, pageSources);
  }

  const worker = new Worker("/starpdf.worker.js", { type: "module" });
  try {
    await sendRequest(worker, { type: "init", id: requestId("init") });
    const buffers = safeInputs.map((input) => input.slice().buffer as ArrayBuffer);
    const response = await sendRequest(
      worker,
      { type: "mergeDocuments", id: requestId("mergeDocuments"), buffers, pageSources },
      buffers
    );
    if (response.type !== "mergeDocuments") {
      throw new Error("Unexpected StarPDF merge response");
    }
    return response.bytes;
  } finally {
    worker.terminate();
  }
}

export async function splitStarPdfDocument(
  input: Uint8Array,
  ranges: { start: number; endExclusive: number }[]
): Promise<Uint8Array[]> {
  if (typeof Worker === "undefined") {
    const handle = await StarPdfClient.open(input);
    const outputs = await handle.splitDocument(ranges);
    await handle.close();
    return outputs;
  }

  const worker = new Worker("/starpdf.worker.js", { type: "module" });
  let handle: number | undefined;
  try {
    await sendRequest(worker, { type: "init", id: requestId("init") });
    const buffer = input.slice().buffer as ArrayBuffer;
    const opened = await sendRequest(
      worker,
      { type: "open", id: requestId("open"), buffer },
      [buffer]
    );
    if (opened.type !== "open") throw new Error("Unexpected StarPDF worker response");
    handle = opened.handle;
    const response = await sendRequest(worker, {
      type: "splitDocument",
      id: requestId("splitDocument"),
      handle,
      ranges,
    });
    if (response.type !== "splitDocument") {
      throw new Error("Unexpected StarPDF split response");
    }
    return response.outputs;
  } finally {
    if (handle !== undefined) {
      try {
        await sendRequest(worker, { type: "close", id: requestId("close"), handle });
      } catch {
        // Terminating the dedicated worker still releases its private registry.
      }
    }
    worker.terminate();
  }
}
