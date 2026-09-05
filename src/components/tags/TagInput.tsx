import type React from "react";
import { useEffect, useRef, useState } from "react";
import type { Tag } from "../../types";

interface TagInputProps {
  tags: Tag[];
  allTags: Tag[];
  onAdd: (name: string) => void;
  onRemove: (tagId: string) => void;
}

export function TagInput({ tags, allTags, onAdd, onRemove }: TagInputProps): React.JSX.Element {
  const [inputValue, setInputValue] = useState("");
  const [showSuggestions, setShowSuggestions] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const existingIds = new Set(tags.map((t) => t.id));
  const trimmed = inputValue.trim().toLowerCase();

  const suggestions = allTags.filter(
    (t) => !existingIds.has(t.id) && t.name.toLowerCase().includes(trimmed),
  );

  const commit = (name: string): void => {
    const clean = name.trim();
    if (clean.length === 0) return;
    const alreadyExists = tags.some((t) => t.name.toLowerCase() === clean.toLowerCase());
    if (!alreadyExists) onAdd(clean);
    setInputValue("");
    setShowSuggestions(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>): void => {
    if (e.key === "Enter" || e.key === ",") {
      e.preventDefault();
      commit(inputValue);
    } else if (e.key === "Escape") {
      setInputValue("");
      setShowSuggestions(false);
      inputRef.current?.blur();
    } else if (e.key === "Backspace" && inputValue === "" && tags.length > 0) {
      const last = tags[tags.length - 1];
      if (last !== undefined) onRemove(last.id);
    }
  };

  useEffect(() => {
    const handler = (e: MouseEvent): void => {
      if (containerRef.current !== null && !containerRef.current.contains(e.target as Node)) {
        setShowSuggestions(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => {
      document.removeEventListener("mousedown", handler);
    };
  }, []);

  return (
    <div ref={containerRef} className="relative">
      <div
        className="flex min-h-[28px] flex-wrap items-center gap-[6px]"
        onClick={() => {
          inputRef.current?.focus();
        }}
      >
        {tags.map((tag) => (
          <span
            key={tag.id}
            className="inline-flex items-center gap-[4px] rounded-full bg-accent/10 px-[8px] py-[2px] font-mono text-[10px] tracking-[0.04em] text-accent"
          >
            {tag.name}
            <button
              type="button"
              onClick={(e) => {
                e.stopPropagation();
                onRemove(tag.id);
              }}
              className="cursor-pointer leading-none text-accent/60 transition-colors hover:text-accent"
              aria-label={`Remove tag ${tag.name}`}
            >
              ×
            </button>
          </span>
        ))}

        <input
          ref={inputRef}
          type="text"
          value={inputValue}
          onChange={(e) => {
            setInputValue(e.target.value);
            setShowSuggestions(true);
          }}
          onFocus={() => {
            setShowSuggestions(true);
          }}
          onKeyDown={handleKeyDown}
          placeholder={tags.length === 0 ? "Add tag…" : ""}
          className="min-w-[80px] flex-1 bg-transparent font-mono text-[11px] text-ink placeholder-ink-4 outline-none"
        />
      </div>

      {showSuggestions && trimmed.length > 0 && suggestions.length > 0 && (
        <div className="absolute top-full left-0 z-10 mt-1 max-h-[160px] overflow-y-auto rounded-[8px] border border-line bg-paper shadow-[0_4px_16px_rgba(10,10,10,0.12)]">
          {suggestions.map((tag) => (
            <button
              key={tag.id}
              type="button"
              onMouseDown={(e) => {
                e.preventDefault();
                commit(tag.name);
              }}
              className="flex w-full cursor-pointer items-center px-3 py-[7px] font-mono text-[11px] text-ink-2 transition-colors hover:bg-paper-sunken hover:text-ink"
            >
              {tag.name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
