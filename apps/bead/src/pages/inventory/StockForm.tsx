import { useId, useState, type FormEvent } from "react";

import {
  MAX_BEADS,
  MAX_CODE_LENGTH,
  MAX_NAME_LENGTH,
  normalizeCode,
  normalizeHex,
} from "../../stores/inventory.ts";
import type { InventoryEntry } from "../../stores/types.ts";

/**
 * D-INV-6: only `beads` is editable in place. Changing a code means deleting
 * the row and re-entering it, because the code is the identity key and an
 * in-place rename would need merge semantics this version does not have.
 *
 * Errors are inline and per field (D-UI-4 / §7): the entry never reaches the
 * store while one is showing, and nothing is silently merged.
 */
export function StockForm({
  codes,
  onAdd,
}: {
  codes: readonly string[];
  onAdd: (entry: InventoryEntry) => void;
}) {
  const fieldId = useId();
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [hex, setHex] = useState("");
  const [beads, setBeads] = useState("");
  const [errors, setErrors] = useState<Record<string, string>>({});

  function validate(): { entry: InventoryEntry } | { errors: Record<string, string> } {
    const next: Record<string, string> = {};
    const normalizedCode = normalizeCode(code);
    if (normalizedCode === "") next["code"] = "请填写色号";
    else if (normalizedCode.length > MAX_CODE_LENGTH)
      next["code"] = `色号最多 ${MAX_CODE_LENGTH} 个字符`;
    else if (codes.includes(normalizedCode)) next["code"] = "色号已存在，请直接修改颗数";

    const trimmedName = name.trim();
    if (trimmedName.length > MAX_NAME_LENGTH) next["name"] = `名称最多 ${MAX_NAME_LENGTH} 个字符`;

    const normalizedHex = normalizeHex(hex);
    if (normalizedHex === null) next["hex"] = "色值要写成 #rrggbb 这样的六位十六进制";

    const parsedBeads = Number(beads.trim());
    if (
      beads.trim() === "" ||
      !Number.isInteger(parsedBeads) ||
      parsedBeads < 0 ||
      parsedBeads > MAX_BEADS
    ) {
      next["beads"] = `颗数要填 0 到 ${MAX_BEADS} 的整数`;
    }

    if (Object.keys(next).length > 0) return { errors: next };
    return {
      entry: { code: normalizedCode, name: trimmedName, hex: normalizedHex!, beads: parsedBeads },
    };
  }

  function handleSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    const result = validate();
    if ("errors" in result) {
      setErrors(result.errors);
      return;
    }
    setErrors({});
    onAdd(result.entry);
    setCode("");
    setName("");
    setHex("");
    setBeads("");
  }

  function field(key: string, label: string, value: string, onChange: (next: string) => void) {
    const inputId = `${fieldId}-${key}`;
    const errorId = `${inputId}-error`;
    const error = errors[key];
    return (
      <p className="stock-form__field">
        <label htmlFor={inputId}>{label}</label>
        <input
          id={inputId}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          aria-invalid={error === undefined ? undefined : true}
          aria-describedby={error === undefined ? undefined : errorId}
        />
        {error !== undefined && (
          <span className="stock-form__error" id={errorId} role="alert">
            {error}
          </span>
        )}
      </p>
    );
  }

  return (
    <form className="stock-form" onSubmit={handleSubmit} aria-label="录入库存">
      {field("code", "色号", code, setCode)}
      {field("name", "名称", name, setName)}
      {field("hex", "色值", hex, setHex)}
      {field("beads", "颗数", beads, setBeads)}
      <button className="button button--primary" type="submit">
        加入库存
      </button>
    </form>
  );
}
