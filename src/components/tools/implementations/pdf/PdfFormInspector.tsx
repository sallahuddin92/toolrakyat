"use client";

import { useId } from "react";
import { type AcroFormField } from "@/lib/pdf/pdf-types";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import { Badge } from "@/components/ui/badge";
import { RotateCcw, FileEdit } from "lucide-react";
import { cn } from "@/lib/utils";

interface PdfFormInspectorProps {
  fields: AcroFormField[];
  fieldValues: Record<string, string | boolean | string[]>;
  onFieldValueChange: (name: string, value: string | boolean | string[]) => void;
  onResetForm: () => void;
  isModified: boolean;
  className?: string;
}

export function PdfFormInspector({
  fields,
  fieldValues,
  onFieldValueChange,
  onResetForm,
  isModified,
  className = "",
}: PdfFormInspectorProps) {
  const baseId = useId();

  if (fields.length === 0) {
    return (
      <aside
        className={cn(
          "w-80 shrink-0 border-l border-slate-200 bg-white p-6 flex flex-col justify-between overflow-y-auto max-h-[750px]",
          className,
        )}
        aria-label="Form Fields Inspector"
      >
        <div className="space-y-4 text-center my-auto">
          <div className="size-12 rounded-2xl bg-slate-100 text-slate-400 flex items-center justify-center mx-auto">
            <FileEdit className="size-6" />
          </div>
          <div className="space-y-1">
            <h4 className="text-sm font-semibold text-slate-800">No AcroForm Fields</h4>
            <p className="text-xs text-slate-500 max-w-[220px] mx-auto leading-relaxed">
              This document does not contain interactive form widgets. You can still navigate pages and export copies.
            </p>
          </div>
        </div>
      </aside>
    );
  }

  return (
    <aside
      className={cn(
        "w-80 shrink-0 border-l border-slate-200 bg-white p-4 flex flex-col justify-between overflow-y-auto max-h-[750px] space-y-4",
        className,
      )}
      aria-label="Form Fields Inspector"
    >
      <div className="space-y-4">
        {/* Header */}
        <div className="flex items-center justify-between pb-2 border-b border-slate-100">
          <div>
            <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-1.5">
              <FileEdit className="size-4 text-sky-600" />
              Form Fields
            </h3>
            <p className="text-xs text-slate-500">{fields.length} interactive fields detected</p>
          </div>
          {isModified && (
            <Badge variant="outline" className="text-[10px] text-amber-700 bg-amber-50 border-amber-200">
              Edited
            </Badge>
          )}
        </div>

        {/* Fields List */}
        <div className="space-y-4 pr-1">
          {fields.map((field, index) => {
            const inputId = `${baseId}-${field.name}-${index}`;
            const currentValue = fieldValues[field.name] ?? field.value;

            return (
              <div
                key={field.name}
                className="space-y-1.5 p-3 rounded-xl bg-slate-50 border border-slate-200/80 transition-all hover:border-slate-300"
              >
                <div className="flex items-center justify-between">
                  <Label
                    htmlFor={inputId}
                    className="text-xs font-medium text-slate-800 truncate max-w-[180px]"
                    title={field.name}
                  >
                    {field.name}
                    {field.isRequired && <span className="text-red-500 ml-0.5">*</span>}
                  </Label>
                  <span className="text-[10px] uppercase font-mono px-1.5 py-0.5 rounded bg-slate-200/70 text-slate-600">
                    {field.type}
                  </span>
                </div>

                {/* Text Field */}
                {field.type === "text" && (
                  <Input
                    id={inputId}
                    type="text"
                    value={typeof currentValue === "string" ? currentValue : ""}
                    onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                    disabled={field.isReadOnly}
                    className="h-8 text-xs bg-white"
                    placeholder="Enter text..."
                  />
                )}

                {/* Checkbox Field */}
                {field.type === "checkbox" && (
                  <div className="flex items-center gap-2 pt-1">
                    <Checkbox
                      id={inputId}
                      checked={Boolean(currentValue)}
                      onCheckedChange={(checked) => onFieldValueChange(field.name, Boolean(checked))}
                      disabled={field.isReadOnly}
                    />
                    <label htmlFor={inputId} className="text-xs text-slate-600 cursor-pointer select-none">
                      {Boolean(currentValue) ? "Checked" : "Unchecked"}
                    </label>
                  </div>
                )}

                {/* Radio Group Field */}
                {field.type === "radio" && field.options && (
                  <div className="space-y-1 pt-1">
                    {field.options.map((opt) => {
                      const radioId = `${inputId}-${opt}`;
                      const isSelected = currentValue === opt;
                      return (
                        <div key={opt} className="flex items-center gap-2">
                          <input
                            type="radio"
                            id={radioId}
                            name={field.name}
                            value={opt}
                            checked={isSelected}
                            onChange={() => onFieldValueChange(field.name, opt)}
                            disabled={field.isReadOnly}
                            className="size-3.5 text-sky-600 focus:ring-sky-500 border-slate-300"
                          />
                          <label htmlFor={radioId} className="text-xs text-slate-600 cursor-pointer select-none">
                            {opt}
                          </label>
                        </div>
                      );
                    })}
                  </div>
                )}

                {/* Dropdown Field */}
                {field.type === "dropdown" && field.options && (
                  <select
                    id={inputId}
                    value={typeof currentValue === "string" ? currentValue : ""}
                    onChange={(e) => onFieldValueChange(field.name, e.target.value)}
                    disabled={field.isReadOnly}
                    className="w-full h-8 rounded-md border border-slate-200 bg-white px-2 text-xs text-slate-800 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500"
                  >
                    <option value="">(Select an option)</option>
                    {field.options.map((opt) => (
                      <option key={opt} value={opt}>
                        {opt}
                      </option>
                    ))}
                  </select>
                )}

                {/* Option List Field */}
                {field.type === "optionList" && field.options && (
                  <select
                    id={inputId}
                    multiple
                    value={Array.isArray(currentValue) ? currentValue : []}
                    onChange={(e) => {
                      const selected = Array.from(e.target.selectedOptions, (o) => o.value);
                      onFieldValueChange(field.name, selected);
                    }}
                    disabled={field.isReadOnly}
                    className="w-full h-20 rounded-md border border-slate-200 bg-white p-1 text-xs text-slate-800 focus:border-sky-500 focus:outline-none focus:ring-1 focus:ring-sky-500"
                  >
                    {field.options.map((opt) => (
                      <option key={opt} value={opt}>
                        {opt}
                      </option>
                    ))}
                  </select>
                )}

                {/* Unsupported Field */}
                {field.type === "unsupported" && (
                  <p className="text-[11px] text-slate-400 italic">Unsupported widget type</p>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Footer Reset Control */}
      <div className="pt-3 border-t border-slate-100 flex items-center justify-between">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onResetForm}
          disabled={!isModified}
          className="text-xs text-slate-600 gap-1.5 w-full"
        >
          <RotateCcw className="size-3.5" />
          Reset Form to Original
        </Button>
      </div>
    </aside>
  );
}
