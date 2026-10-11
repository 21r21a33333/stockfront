import { useEffect, useLayoutEffect, useRef, type KeyboardEvent } from "react";

/**
 * A chat-style prompt: one line until the text wraps, then it grows to a
 * few lines and scrolls. Enter submits the surrounding form; Shift+Enter
 * starts a new line.
 */
export function PromptInput({
  id,
  value,
  onChange,
  placeholder,
  maxLength,
  className,
}: {
  id: string;
  value: string;
  onChange(value: string): void;
  placeholder?: string;
  maxLength?: number;
  className?: string;
}) {
  const field = useRef<HTMLTextAreaElement>(null);

  // Measure after every value change, including ones set from outside
  // (example chips, refinements), so the height always matches the text.
  useLayoutEffect(() => fitHeight(field.current), [value]);

  // A narrower field wraps the same text onto more lines.
  useEffect(() => {
    const element = field.current;
    if (!element) return;
    let width = element.clientWidth;
    const observer = new ResizeObserver(() => {
      if (element.clientWidth === width) return;
      width = element.clientWidth;
      fitHeight(element);
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  function onKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (
      event.key === "Enter" &&
      !event.shiftKey &&
      !event.nativeEvent.isComposing
    ) {
      event.preventDefault();
      event.currentTarget.form?.requestSubmit();
    }
  }

  return (
    <textarea
      ref={field}
      id={id}
      rows={1}
      className={className}
      value={value}
      maxLength={maxLength}
      placeholder={placeholder}
      onChange={(event) => onChange(event.target.value)}
      onKeyDown={onKeyDown}
    />
  );
}

function fitHeight(element: HTMLTextAreaElement | null) {
  if (!element) return;
  element.style.height = "auto";
  element.style.height = `${element.scrollHeight}px`;
}
