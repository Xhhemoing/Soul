import {
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type ChangeEvent,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { useNavigate } from "react-router";

import { decodeImage, MAX_SOURCE_SIDE } from "../../algo/decode.ts";
import { BOARD_28, BOARD_56, type Framing } from "../../algo/framing.ts";
import { occupiedCount } from "../../algo/grid.ts";
import { AlgoError, type RgbaImage } from "../../algo/image.ts";
import { imageToPattern, type PipelineOptions, type PipelineResult } from "../../algo/pipeline.ts";
import type { ImageKind } from "../../algo/classify.ts";
import { mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { createProjectFromConversion } from "../../stores/projects.ts";
import { useStore } from "../../stores/store.tsx";
import type { ProjectStatus } from "../../stores/types.ts";
import { ConversionPreview } from "./ConversionPreview.tsx";

/**
 * WP-B03: the `/create?entry=upload` workbench.
 *
 * D-UP-1 keeps it in the entry panel rather than behind a new route: the
 * selected entry is already in the URL (D-UI-1), so this is linkable and
 * backable without one. D-UP-8 runs the pipeline on the main thread with a busy
 * state and defers the worker (R-UP-1) — the pipeline is a pure function of
 * `(image, options)`, so moving it later breaks no API.
 *
 * D-UP-8, second half: nothing here reaches storage. The preview is derived
 * data; the first byte is written when one of the two CTAs is pressed.
 */

const ACCEPTED_TYPES = ["image/png", "image/jpeg"];
const ACCEPTED_EXTENSIONS = [".png", ".jpg", ".jpeg"];
const FILE_TYPE_ERROR = "只支持 png / jpg";
const SAVE_ERROR = "转换结果没有保存";
const UNTITLED = "未命名转换";
const MAX_TITLE_LENGTH = 64;
/** D-UP-7: v0 boards are square and ≤56 per axis; the DOM canvas is sized for it. */
const MAX_OUTPUT_CELLS = BOARD_56;
/** Enough to swallow a slider drag without making a click feel laggy. */
const RECOMPUTE_DELAY_MS = 60;

type KindChoice = "auto" | ImageKind;
type FramingChoice = "board" | "aspect" | "manual";

/** One pipeline run, kept next to the inputs that produced it. */
interface Computation {
  readonly image: RgbaImage;
  readonly options: PipelineOptions;
  readonly result: PipelineResult | null;
  readonly error: string | null;
}

const KIND_LABEL: Record<ImageKind, string> = { PixelArt: "像素图", Photo: "照片" };

function isAcceptedFile(file: File): boolean {
  if (file.type !== "") return ACCEPTED_TYPES.includes(file.type);
  // Some pickers hand over an empty MIME type; the extension is the fallback,
  // and anything else is refused out loud rather than quietly ignored.
  const name = file.name.toLowerCase();
  return ACCEPTED_EXTENSIONS.some((extension) => name.endsWith(extension));
}

function titleFromFileName(name: string): string {
  const withoutExtension = name.replace(/\.[^.]+$/, "").trim();
  return withoutExtension.slice(0, MAX_TITLE_LENGTH) || UNTITLED;
}

function describeDecodeError(error: unknown): string {
  if (error instanceof AlgoError && error.code === "SourceTooLarge") {
    return `图片太大：最长边不能超过 ${MAX_SOURCE_SIDE} 像素`;
  }
  return "这张图片解不开，换一张试试";
}

function createPreviewUrl(file: File): string | null {
  const factory = globalThis.URL?.createObjectURL;
  if (typeof factory !== "function") return null;
  return factory.call(globalThis.URL, file);
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

export function UploadWorkbench() {
  const fieldId = useId();
  const navigate = useNavigate();
  const { addProject, savePatternDoc } = useStore();

  const [fileName, setFileName] = useState<string | null>(null);
  const [fileError, setFileError] = useState<string | null>(null);
  const [image, setImage] = useState<RgbaImage | null>(null);
  const [previewUrl, setPreviewUrl] = useState<string | null>(null);

  const [kindChoice, setKindChoice] = useState<KindChoice>("auto");
  const [dither, setDither] = useState(false);
  const [framingChoice, setFramingChoice] = useState<FramingChoice>("board");
  const [boardSize, setBoardSize] = useState<28 | 56>(BOARD_28);
  const [maxSide, setMaxSide] = useState<28 | 56>(BOARD_28);
  const [cropX, setCropX] = useState(0);
  const [cropY, setCropY] = useState(0);
  const [cropSide, setCropSide] = useState(1);
  const [outputCells, setOutputCells] = useState(BOARD_28);

  const [computation, setComputation] = useState<Computation | null>(null);

  const [title, setTitle] = useState("");
  const [saveError, setSaveError] = useState<string | null>(null);
  const [savedNote, setSavedNote] = useState<string | null>(null);
  const saving = useRef(false);
  const dragFrom = useRef<{ x: number; y: number; box: DOMRect } | null>(null);

  useEffect(() => {
    if (previewUrl === null) return undefined;
    // D-INV-11 precedent: the only URL in this file is a runtime `blob:`, and
    // it is handed back as soon as the source image changes.
    return () => {
      globalThis.URL?.revokeObjectURL?.(previewUrl);
    };
  }, [previewUrl]);

  async function handleFile(event: ChangeEvent<HTMLInputElement>): Promise<void> {
    const file = event.target.files?.[0];
    setSaveError(null);
    setSavedNote(null);
    if (file === undefined) return;

    if (!isAcceptedFile(file)) {
      setFileError(FILE_TYPE_ERROR);
      setImage(null);
      setComputation(null);
      setFileName(null);
      setPreviewUrl(null);
      return;
    }

    setFileError(null);
    setFileName(file.name);
    setTitle(titleFromFileName(file.name));
    setPreviewUrl(createPreviewUrl(file));

    try {
      const decoded = await decodeImage(file);
      setImage(decoded);
      const side = Math.min(decoded.width, decoded.height);
      setCropX(0);
      setCropY(0);
      setCropSide(side);
      setOutputCells(Math.min(MAX_OUTPUT_CELLS, side));
    } catch (error) {
      setImage(null);
      setComputation(null);
      setFileError(describeDecodeError(error));
    }
  }

  const framing = useMemo<Framing>(() => {
    if (framingChoice === "board") return { mode: "board", size: boardSize };
    if (framingChoice === "aspect") return { mode: "aspect", maxSide };
    const side = Math.max(1, cropSide);
    return {
      mode: "manual",
      scale: clamp(outputCells, 1, MAX_OUTPUT_CELLS) / side,
      crop: { x: cropX, y: cropY, width: side, height: side },
    };
  }, [framingChoice, boardSize, maxSide, cropSide, cropX, cropY, outputCells]);

  const options = useMemo(
    () => ({
      framing,
      dither,
      ...(kindChoice === "auto" ? {} : { kind: kindChoice }),
    }),
    [framing, dither, kindChoice],
  );

  // D-UP-8: a parameter change re-runs the pipeline, debounced so a slider drag
  // is one conversion and not thirty. The outcome is stored with the inputs it
  // came from, which is what makes 「转换中…」 a derived fact rather than a
  // second piece of state that can disagree with the first.
  useEffect(() => {
    if (image === null) return undefined;
    const timer = setTimeout(() => {
      try {
        setComputation({ image, options, result: imageToPattern(image, options), error: null });
      } catch (error) {
        setComputation({
          image,
          options,
          result: null,
          error: error instanceof AlgoError ? `这组参数转不出网格：${error.message}` : "转换失败",
        });
      }
    }, RECOMPUTE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [image, options]);

  const current =
    computation !== null && computation.image === image && computation.options === options
      ? computation
      : null;
  const busy = image !== null && current === null;
  const result = current?.result ?? null;
  const pipelineError = current?.error ?? null;

  const beads = result === null ? 0 : occupiedCount(result.grid);
  const emptyGrid = result !== null && beads === 0;
  const canSave = result !== null && !emptyGrid && !busy;

  async function save(status: Extract<ProjectStatus, "todo" | "active">): Promise<void> {
    if (result === null || saving.current) return;
    saving.current = true;
    setSaveError(null);
    setSavedNote(null);

    const projectId = mintProjectId();
    const doc = createPatternDoc(projectId, result.grid, {
      kind: result.kind,
      ditherApplied: result.ditherApplied,
    });

    try {
      // D-UP-10: the document first, awaited. A project whose grid is not on
      // disk is a project that opens to an empty board, so it is never minted
      // until this resolves.
      await savePatternDoc(doc);
    } catch {
      setSaveError(SAVE_ERROR);
      saving.current = false;
      return;
    }

    const trimmed = title.trim().slice(0, MAX_TITLE_LENGTH) || UNTITLED;
    addProject(createProjectFromConversion(projectId, trimmed, status));
    saving.current = false;

    if (status === "active") {
      void navigate("/workspace");
      return;
    }
    setSavedNote(`已加入待拼：${trimmed}`);
  }

  function moveCrop(dx: number, dy: number): void {
    if (image === null) return;
    setCropX((x) => clamp(x + dx, 0, Math.max(0, image.width - cropSide)));
    setCropY((y) => clamp(y + dy, 0, Math.max(0, image.height - cropSide)));
  }

  function panCrop(event: KeyboardEvent<HTMLDivElement>): void {
    const step = event.shiftKey ? 10 : 1;
    const deltas: Record<string, [number, number]> = {
      ArrowLeft: [-step, 0],
      ArrowRight: [step, 0],
      ArrowUp: [0, -step],
      ArrowDown: [0, step],
    };
    const delta = deltas[event.key];
    if (delta === undefined) return;
    event.preventDefault();
    moveCrop(delta[0], delta[1]);
  }

  // D-UP-7's other half. Pointer events rather than mouse ones so a touch drag
  // is the same code path, and the displayed box is measured rather than
  // assumed: the preview is CSS-scaled to the viewport, so a screen pixel is
  // not a source pixel.
  function dragCrop(event: PointerEvent<HTMLDivElement>): void {
    if (image === null || dragFrom.current !== null) return;
    const box = event.currentTarget.parentElement?.getBoundingClientRect();
    if (box === undefined || box.width === 0 || box.height === 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragFrom.current = { x: event.clientX, y: event.clientY, box };
  }

  function dragCropMove(event: PointerEvent<HTMLDivElement>): void {
    const from = dragFrom.current;
    if (from === null || image === null) return;
    const dx = Math.round(((event.clientX - from.x) / from.box.width) * image.width);
    const dy = Math.round(((event.clientY - from.y) / from.box.height) * image.height);
    if (dx === 0 && dy === 0) return;
    dragFrom.current = { ...from, x: event.clientX, y: event.clientY };
    moveCrop(dx, dy);
  }

  function dragCropEnd(): void {
    dragFrom.current = null;
  }

  const pixelArtPath = result !== null ? result.kind === "PixelArt" : kindChoice === "PixelArt";

  return (
    <div className="upload">
      <p className="upload__field">
        <label htmlFor={`${fieldId}-file`}>选择图片（png / jpg）</label>
        <input
          id={`${fieldId}-file`}
          type="file"
          accept="image/png,image/jpeg"
          onChange={(event) => void handleFile(event)}
        />
        {fileError !== null && (
          <span className="stock-form__error" role="alert">
            {fileError}
          </span>
        )}
      </p>

      {image !== null && (
        <div className="upload__panel" aria-label="转换参数" role="group" aria-busy={busy}>
          <fieldset className="upload__group">
            <legend>识别路径</legend>
            {(
              [
                ["auto", "自动"],
                ["PixelArt", "像素图"],
                ["Photo", "照片"],
              ] as const
            ).map(([value, label]) => (
              <label key={value}>
                <input
                  type="radio"
                  name={`${fieldId}-kind`}
                  checked={kindChoice === value}
                  onChange={() => setKindChoice(value)}
                />
                {label}
              </label>
            ))}
          </fieldset>

          <p className="upload__field">
            <label>
              <input
                type="checkbox"
                checked={dither}
                disabled={pixelArtPath}
                onChange={(event) => setDither(event.target.checked)}
              />
              抖动（Floyd–Steinberg）
            </label>
            {pixelArtPath && <span className="stub-note">像素图路径不做抖动</span>}
          </p>

          <fieldset className="upload__group">
            <legend>框定方式</legend>
            {(
              [
                ["board", "固定板"],
                ["aspect", "按比例适配"],
                ["manual", "手动视口"],
              ] as const
            ).map(([value, label]) => (
              <label key={value}>
                <input
                  type="radio"
                  name={`${fieldId}-framing`}
                  checked={framingChoice === value}
                  onChange={() => setFramingChoice(value)}
                />
                {label}
              </label>
            ))}
          </fieldset>

          {framingChoice === "board" && (
            <p className="upload__field">
              <label htmlFor={`${fieldId}-board`}>板子尺寸</label>
              <select
                id={`${fieldId}-board`}
                value={boardSize}
                onChange={(event) => setBoardSize(Number(event.target.value) === BOARD_56 ? BOARD_56 : BOARD_28)}
              >
                <option value={BOARD_28}>28×28</option>
                <option value={BOARD_56}>56×56</option>
              </select>
            </p>
          )}

          {framingChoice === "aspect" && (
            <p className="upload__field">
              <label htmlFor={`${fieldId}-maxside`}>最长边格数</label>
              <select
                id={`${fieldId}-maxside`}
                value={maxSide}
                onChange={(event) => setMaxSide(Number(event.target.value) === BOARD_56 ? BOARD_56 : BOARD_28)}
              >
                <option value={BOARD_28}>28</option>
                <option value={BOARD_56}>56</option>
              </select>
            </p>
          )}

          {framingChoice === "manual" && (
            <div className="upload__manual">
              {previewUrl !== null && (
                <div className="upload__viewport">
                  <img src={previewUrl} alt="源图预览" />
                  <div
                    className="upload__crop"
                    role="group"
                    aria-label="取景框：拖拽或方向键平移"
                    tabIndex={0}
                    onKeyDown={panCrop}
                    onPointerDown={dragCrop}
                    onPointerMove={dragCropMove}
                    onPointerUp={dragCropEnd}
                    onPointerCancel={dragCropEnd}
                    style={{
                      left: `${(cropX / image.width) * 100}%`,
                      top: `${(cropY / image.height) * 100}%`,
                      width: `${(cropSide / image.width) * 100}%`,
                      height: `${(cropSide / image.height) * 100}%`,
                    }}
                  />
                </div>
              )}
              <p className="upload__field">
                <label htmlFor={`${fieldId}-side`}>取景框边长（源像素）</label>
                <input
                  id={`${fieldId}-side`}
                  type="number"
                  min={1}
                  max={Math.min(image.width, image.height)}
                  value={cropSide}
                  onChange={(event) => {
                    const side = clamp(
                      Math.trunc(Number(event.target.value)) || 1,
                      1,
                      Math.min(image.width, image.height),
                    );
                    setCropSide(side);
                    setCropX((x) => clamp(x, 0, image.width - side));
                    setCropY((y) => clamp(y, 0, image.height - side));
                  }}
                />
              </p>
              <p className="upload__field">
                <label htmlFor={`${fieldId}-cells`}>输出格数</label>
                <input
                  id={`${fieldId}-cells`}
                  type="range"
                  min={1}
                  max={MAX_OUTPUT_CELLS}
                  value={outputCells}
                  onChange={(event) =>
                    // D-UP-7: the ≤56 ceiling is clamped here rather than
                    // refused later — the DOM canvas and the v0 square boards
                    // both stop there.
                    setOutputCells(clamp(Math.trunc(Number(event.target.value)), 1, MAX_OUTPUT_CELLS))
                  }
                />
                <span>{outputCells} 格</span>
              </p>
            </div>
          )}
        </div>
      )}

      {busy && <p className="stub-note">转换中…</p>}
      {pipelineError !== null && (
        <p className="stock-form__error" role="alert">
          {pipelineError}
        </p>
      )}

      {result !== null && (
        <>
          <ConversionPreview
            grid={result.grid}
            bom={result.bom}
            palette={result.palette}
            kindLabel={KIND_LABEL[result.kind]}
            confidence={result.classification.confidence}
            ditherApplied={result.ditherApplied}
          />

          {emptyGrid && (
            <p className="upload__empty" role="alert">
              这张图转出来全是空格，没有可拼的豆子，换一张或改一下框定
            </p>
          )}

          <p className="upload__field">
            <label htmlFor={`${fieldId}-title`}>项目名称</label>
            <input
              id={`${fieldId}-title`}
              value={title}
              maxLength={MAX_TITLE_LENGTH}
              onChange={(event) => setTitle(event.target.value)}
            />
          </p>

          <div className="detail-actions">
            <button
              className="button"
              type="button"
              disabled={!canSave}
              onClick={() => void save("todo")}
            >
              加入待拼
            </button>
            <button
              className="button button--primary"
              type="button"
              disabled={!canSave}
              onClick={() => void save("active")}
            >
              转入工作台
            </button>
          </div>

          {saveError !== null && (
            <p className="stock-form__error" role="alert">
              {saveError}
            </p>
          )}
          {savedNote !== null && (
            <p className="stub-note" role="status">
              {savedNote}
            </p>
          )}
        </>
      )}

      {fileName !== null && image === null && fileError === null && (
        <p className="stub-note">正在读取 {fileName}…</p>
      )}
    </div>
  );
}
