"use client";

import { useState, useMemo } from "react";
import { Card, CardContent } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Textarea } from "@/components/ui/textarea";
import { Copy, RotateCcw, Check } from "lucide-react";
import toast from "react-hot-toast";

interface TextStats {
  words: number;
  characters: number;
  charactersNoSpaces: number;
  sentences: number;
  paragraphs: number;
  readingTimeMinutes: number;
  speakingTimeMinutes: number;
}

function computeStats(text: string): TextStats {
  const trimmed = text.trim();
  if (!trimmed) {
    return {
      words: 0,
      characters: 0,
      charactersNoSpaces: 0,
      sentences: 0,
      paragraphs: 0,
      readingTimeMinutes: 0,
      speakingTimeMinutes: 0,
    };
  }

  const words = trimmed.split(/\s+/).filter(Boolean).length;
  const characters = text.length;
  const charactersNoSpaces = text.replace(/\s/g, "").length;
  const sentences = trimmed.split(/[.!?]+(?:\s+|$)/).filter(Boolean).length;
  const paragraphs = trimmed.split(/\n\s*\n/).filter((p) => p.trim().length > 0).length;
  
  // Standard reading speed: ~200 words per minute
  const readingTimeMinutes = Math.ceil(words / 200);
  // Standard speaking speed: ~130 words per minute
  const speakingTimeMinutes = Math.ceil(words / 130);

  return {
    words,
    characters,
    charactersNoSpaces,
    sentences,
    paragraphs,
    readingTimeMinutes,
    speakingTimeMinutes,
  };
}

export function WordCounterTool() {
  const [text, setText] = useState("");
  const [copied, setCopied] = useState(false);

  const stats = useMemo(() => computeStats(text), [text]);

  const handleCopy = async () => {
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      toast.success("Text copied to clipboard");
      setTimeout(() => setCopied(false), 2000);
    } catch {
      toast.error("Failed to copy text");
    }
  };

  const handleClear = () => {
    setText("");
    setCopied(false);
  };

  return (
    <div className="space-y-6">
      {/* Metrics Grid */}
      <div className="grid grid-cols-2 gap-3 sm:grid-cols-4 sm:gap-4">
        <Card className="rounded-2xl border-slate-200">
          <CardContent className="p-4 text-center">
            <p className="text-xs font-medium text-slate-500 uppercase tracking-wider">Words</p>
            <p className="mt-1 text-2xl font-bold text-slate-900 tabular-nums">{stats.words}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-slate-200">
          <CardContent className="p-4 text-center">
            <p className="text-xs font-medium text-slate-500 uppercase tracking-wider">Characters</p>
            <p className="mt-1 text-2xl font-bold text-slate-900 tabular-nums">{stats.characters}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-slate-200">
          <CardContent className="p-4 text-center">
            <p className="text-xs font-medium text-slate-500 uppercase tracking-wider">Sentences</p>
            <p className="mt-1 text-2xl font-bold text-slate-900 tabular-nums">{stats.sentences}</p>
          </CardContent>
        </Card>
        <Card className="rounded-2xl border-slate-200">
          <CardContent className="p-4 text-center">
            <p className="text-xs font-medium text-slate-500 uppercase tracking-wider">Paragraphs</p>
            <p className="mt-1 text-2xl font-bold text-slate-900 tabular-nums">{stats.paragraphs}</p>
          </CardContent>
        </Card>
      </div>

      {/* Main Text Input Area */}
      <Card className="rounded-2xl border-slate-200 shadow-xs">
        <CardContent className="p-4 sm:p-6 space-y-4">
          <div className="flex items-center justify-between">
            <label htmlFor="word-counter-input" className="text-sm font-medium text-slate-700">
              Type or paste your text
            </label>
            <div className="flex items-center gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={handleCopy}
                disabled={!text}
                className="gap-1.5 rounded-lg text-xs"
              >
                {copied ? <Check className="size-3.5 text-green-600" /> : <Copy className="size-3.5" />}
                {copied ? "Copied" : "Copy"}
              </Button>
              <Button
                variant="ghost"
                size="sm"
                onClick={handleClear}
                disabled={!text}
                className="gap-1.5 rounded-lg text-xs text-slate-500 hover:text-slate-900"
              >
                <RotateCcw className="size-3.5" />
                Clear
              </Button>
            </div>
          </div>

          <Textarea
            id="word-counter-input"
            rows={10}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Start typing or paste your content here..."
            className="w-full resize-y rounded-xl border-slate-200 p-4 text-base font-normal leading-relaxed focus:border-sky-500 focus:ring-sky-500/20"
          />

          {/* Secondary Details */}
          <div className="flex flex-wrap items-center justify-between gap-4 border-t border-slate-100 pt-3 text-xs text-slate-500">
            <div className="flex flex-wrap gap-4">
              <span>
                Characters (no spaces): <strong className="font-semibold text-slate-700">{stats.charactersNoSpaces}</strong>
              </span>
              <span>
                Est. Reading time: <strong className="font-semibold text-slate-700">{stats.readingTimeMinutes} min</strong>
              </span>
              <span>
                Est. Speaking time: <strong className="font-semibold text-slate-700">{stats.speakingTimeMinutes} min</strong>
              </span>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
