import { NextRequest, NextResponse } from "next/server";

const BACKEND_URL = process.env.BACKEND_URL || "http://localhost:8000";

export async function GET(request: NextRequest, { params }: { params: Promise<{ path?: string[] }> }) {
  const { path: pathSegments } = await params;
  const path = pathSegments?.join("/") || "";
  const searchParams = request.nextUrl.searchParams.toString();
  const url = `${BACKEND_URL}/documents/${path}${searchParams ? `?${searchParams}` : ""}`;

  try {
    const response = await fetch(url);
    if (!response.ok) {
      return NextResponse.json({ error: "Backend error" }, { status: response.status });
    }
    
    // For FileResponse (export)
    const contentType = response.headers.get("content-type");
    if (contentType === "application/pdf" || contentType?.includes("vnd.openxmlformats-officedocument")) {
        const blob = await response.blob();
        return new NextResponse(blob, {
            headers: {
                "Content-Type": contentType,
                "Content-Disposition": response.headers.get("content-disposition") || "",
            }
        });
    }

    const data = await response.json();
    return NextResponse.json({ ...data, success: true });
  } catch (error) {
    console.error("Proxy error:", error);
    return NextResponse.json({ error: "Failed to connect to backend" }, { status: 500 });
  }
}

export async function POST(request: NextRequest, { params }: { params: Promise<{ path?: string[] }> }) {
  const { path: pathSegments } = await params;
  const path = pathSegments?.join("/") || "";
  const searchParams = request.nextUrl.searchParams.toString();
  const url = `${BACKEND_URL}/documents/${path}${searchParams ? `?${searchParams}` : ""}`;

  try {
    const contentType = request.headers.get("content-type");
    let body;
    
    if (contentType?.includes("multipart/form-data")) {
        body = await request.formData();
    } else {
        body = JSON.stringify(await request.json());
    }

    const response = await fetch(url, {
      method: "POST",
      headers: contentType?.includes("application/json") ? { "Content-Type": "application/json" } : {},
      body: body,
    });

    if (!response.ok) {
      const errorText = await response.text();
      return NextResponse.json({ error: errorText || "Backend error" }, { status: response.status });
    }

    const data = await response.json();
    return NextResponse.json({ ...data, success: true });
  } catch (error) {
    console.error("Proxy error:", error);
    return NextResponse.json({ error: "Failed to connect to backend" }, { status: 500 });
  }
}
