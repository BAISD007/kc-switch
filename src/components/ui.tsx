import type { ReactNode } from "react";

export function Button({
  children,
  onClick,
  variant = "secondary",
  size = "md",
  disabled,
  type = "button",
  icon,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md";
  disabled?: boolean;
  type?: "button" | "submit";
  icon?: ReactNode;
}) {
  const base =
    "inline-flex items-center justify-center gap-1.5 rounded-md font-medium whitespace-nowrap transition-[background-color,color,border-color,box-shadow] duration-[var(--duration-fast)] ease-[var(--ease-out)] disabled:pointer-events-none disabled:opacity-40";
  const sizes = {
    sm: "h-7 px-2.5 text-xs",
    md: "h-8 px-3.5 text-[13px]",
  };
  const variants = {
    primary:
      "bg-accent text-ink-on-accent hover:bg-accent-hover shadow-subtle",
    secondary:
      "hairline bg-surface-raised text-ink hover:bg-hover disabled:bg-surface",
    ghost: "text-ink-muted hover:bg-hover hover:text-ink",
    danger:
      "bg-danger-soft text-danger hairline hover:brightness-[0.98]",
  };
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={`${base} ${sizes[size]} ${variants[variant]}`}
    >
      {icon}
      {children}
    </button>
  );
}

export function IconButton({
  children,
  onClick,
  title,
  disabled,
}: {
  children: ReactNode;
  onClick?: () => void;
  title?: string;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      title={title}
      disabled={disabled}
      className="flex h-[26px] w-[26px] items-center justify-center rounded-md text-ink-muted transition-colors duration-[var(--duration-fast)] hover:bg-hover hover:text-ink disabled:pointer-events-none disabled:opacity-40"
    >
      {children}
    </button>
  );
}

export function Field({
  label,
  hint,
  children,
  required,
}: {
  label: string;
  hint?: string;
  children: ReactNode;
  required?: boolean;
}) {
  return (
    <label className="flex flex-col gap-1.5">
      <span className="text-xs font-medium text-ink">
        {label}
        {required && <span className="ml-0.5 text-danger">*</span>}
      </span>
      {children}
      {hint && (
        <span className="text-[11px] leading-relaxed text-ink-faint">{hint}</span>
      )}
    </label>
  );
}

const controlCls =
  "w-full rounded-md border border-line bg-surface-raised px-3 text-[13px] text-ink outline-none transition-[border-color,box-shadow] duration-[var(--duration-fast)] placeholder:text-ink-quaternary hover:border-line-strong focus:border-accent focus:shadow-[0_0_0_3px_var(--color-accent-soft)]";

export function TextInput({
  value,
  onChange,
  placeholder,
  type = "text",
  mono,
  height = "md",
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  type?: "text" | "password" | "number";
  mono?: boolean;
  height?: "sm" | "md";
}) {
  return (
    <input
      type={type}
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className={`${controlCls} ${height === "sm" ? "h-7" : "h-8"} ${
        mono ? "font-mono text-xs" : ""
      }`}
    />
  );
}

export function Select({
  value,
  onChange,
  options,
  allowEmpty,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
  allowEmpty?: boolean;
}) {
  // 箭头用 CSS 类绘制，深浅色各一套（data URI 里的 currentColor 不会被解析）
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className={`${controlCls} select-arrow h-8 cursor-pointer appearance-none pr-8`}
    >
      {allowEmpty && <option value="">（不设置）</option>}
      {options.map((o) => (
        <option key={o.value} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}

export function TextArea({
  value,
  onChange,
  placeholder,
  rows = 3,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  rows?: number;
}) {
  return (
    <textarea
      value={value}
      onChange={(e) => onChange(e.target.value)}
      rows={rows}
      placeholder={placeholder}
      className={`${controlCls} resize-y py-2 font-mono text-xs leading-relaxed`}
    />
  );
}

/** Kimi Code 的复选框：小尺寸、方角、蓝色选中 */
export function Checkbox({
  checked,
  onChange,
  label,
  hint,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  hint?: string;
}) {
  return (
    <label
      className={`flex cursor-pointer items-start gap-2.5 rounded-md border px-3 py-2 transition-[background-color,border-color] duration-[var(--duration-fast)] ${
        checked
          ? "border-accent-bd bg-accent-soft"
          : "border-line bg-surface-raised hover:border-line-strong hover:bg-hover"
      }`}
    >
      <span
        className={`mt-[1px] flex h-4 w-4 shrink-0 items-center justify-center rounded-[3px] border transition-colors ${
          checked ? "border-accent bg-accent" : "border-line-strong bg-surface-raised"
        }`}
      >
        {checked && (
          <svg
            viewBox="0 0 12 12"
            className="h-2.5 w-2.5 text-white"
            fill="none"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M2.5 6.2 4.8 8.5 9.5 3.8" />
          </svg>
        )}
      </span>
      <input
        type="checkbox"
        checked={checked}
        onChange={(e) => onChange(e.target.checked)}
        className="sr-only"
      />
      <span className="flex min-w-0 flex-col gap-0.5">
        <span className="text-xs font-medium text-ink">{label}</span>
        {hint && (
          <span className="text-[11px] leading-relaxed text-ink-faint">{hint}</span>
        )}
      </span>
    </label>
  );
}

export function Toggle({
  checked,
  onChange,
  label,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label?: string;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className="group inline-flex items-center gap-2"
      aria-pressed={checked}
    >
      <span
        className={`relative inline-flex h-[18px] w-8 shrink-0 items-center rounded-full transition-colors duration-[var(--duration-base)] ${
          checked ? "bg-accent" : "bg-ink-quaternary"
        }`}
      >
        <span
          className={`absolute h-3.5 w-3.5 rounded-full bg-white shadow-subtle transition-transform duration-[var(--duration-base)] ease-[var(--ease-out)] ${
            checked ? "translate-x-[15px]" : "translate-x-[2px]"
          }`}
        />
      </span>
      {label && <span className="text-xs text-ink-muted">{label}</span>}
    </button>
  );
}

export function Tag({
  children,
  tone = "neutral",
  title,
}: {
  children: ReactNode;
  tone?: "neutral" | "accent" | "success" | "attention" | "danger";
  title?: string;
}) {
  const tones = {
    neutral: "bg-subtle text-ink-muted",
    accent: "bg-accent-soft text-accent",
    success: "bg-success-soft text-success",
    attention: "bg-attention-soft text-attention",
    danger: "bg-danger-soft text-danger",
  };
  return (
    <span
      title={title}
      className={`inline-flex shrink-0 items-center rounded px-1.5 py-[1px] text-[10px] font-medium leading-4 ${tones[tone]}`}
    >
      {children}
    </span>
  );
}

export function Modal({
  title,
  subtitle,
  children,
  footer,
  onClose,
  wide,
}: {
  title: string;
  subtitle?: string;
  children: ReactNode;
  footer: ReactNode;
  onClose: () => void;
  wide?: boolean;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-scrim p-6 backdrop-blur-[2px]"
      onClick={onClose}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        className={`flex max-h-full w-full flex-col overflow-hidden rounded-xl bg-surface-overlay shadow-modal ${
          wide ? "max-w-3xl" : "max-w-lg"
        }`}
        style={{ border: "var(--p-hairline) solid var(--color-line)" }}
      >
        <div className="flex min-w-0 items-start justify-between gap-4 border-b border-line px-5 py-4">
          <div className="min-w-0">
            <h2 className="text-[15px] font-semibold text-ink">{title}</h2>
            {subtitle && (
              <p className="mt-1 text-xs leading-relaxed text-ink-muted">
                {subtitle}
              </p>
            )}
          </div>
          <IconButton onClick={onClose} title="关闭">
            <svg
              viewBox="0 0 16 16"
              className="h-3.5 w-3.5"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.6"
              strokeLinecap="round"
            >
              <path d="M4 4l8 8M12 4l-8 8" />
            </svg>
          </IconButton>
        </div>
        <div className="flex-1 overflow-y-auto px-5 py-4">{children}</div>
        <div className="flex items-center gap-2 border-t border-line bg-surface-sunken px-5 py-3">
          {footer}
        </div>
      </div>
    </div>
  );
}

export function ErrorBar({ message }: { message: string }) {
  return (
    <div className="mx-7 mt-3 flex items-start gap-2 rounded-md border border-danger-bd bg-danger-soft px-3 py-2">
      <span className="mt-[1px] shrink-0 text-danger">
        <svg
          viewBox="0 0 16 16"
          className="h-3.5 w-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
        >
          <circle cx="8" cy="8" r="6.2" />
          <path d="M8 5v3.5M8 11h.01" />
        </svg>
      </span>
      <span className="text-xs leading-relaxed text-danger">{message}</span>
    </div>
  );
}

export function Empty({ text, action }: { text: string; action?: ReactNode }) {
  return (
    <div className="flex flex-1 flex-col items-center justify-center gap-3 py-16">
      <div className="flex h-10 w-10 items-center justify-center rounded-full bg-surface text-ink-quaternary">
        <svg
          viewBox="0 0 20 20"
          className="h-5 w-5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.4"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <rect x="3.5" y="4.5" width="13" height="11" rx="2" />
          <path d="M3.5 8.5h13" />
        </svg>
      </div>
      <span className="text-[13px] text-ink-faint">{text}</span>
      {action}
    </div>
  );
}

export function Spinner({ label }: { label?: string }) {
  return (
    <div className="flex flex-col items-center gap-2.5 py-12">
      <span className="h-4 w-4 animate-spin rounded-full border-[1.5px] border-line border-t-accent [animation-duration:0.7s]" />
      {label && <span className="text-xs text-ink-faint">{label}</span>}
    </div>
  );
}

/** 统一的区块标题（Kimi Code 风格：小字、中灰、字重 500） */
export function SectionLabel({
  children,
  hint,
}: {
  children: ReactNode;
  hint?: string;
}) {
  return (
    <div className="mb-2 flex items-baseline justify-between gap-3">
      <span className="text-xs font-medium text-ink">{children}</span>
      {hint && <span className="text-[11px] text-ink-faint">{hint}</span>}
    </div>
  );
}
