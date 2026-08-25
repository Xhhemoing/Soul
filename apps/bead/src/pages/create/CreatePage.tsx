import { Link, useSearchParams } from "react-router";

import { Card } from "../../components/Card.tsx";
import { useDocumentTitle } from "../../app/useDocumentTitle.ts";

/**
 * R6 in the round-1 IA review: before WP-B03 and WP-B07 land these entries are
 * explanation panels, not features, and every panel names the work package
 * that owns it so a tester reads "not built yet" instead of filing a bug.
 *
 * The selected entry lives in `?entry=` rather than component state (D-UI-1),
 * so no new route is needed to make a stub explanation linkable and backable.
 */
interface CreateEntry {
  key: string;
  label: string;
  /** Null when the entry already works in B02. */
  owner: string | null;
  detail: string;
  action?: { label: string; to: string };
}

const ENTRIES: CreateEntry[] = [
  {
    key: "upload",
    label: "上传图片转豆图",
    owner: "WP-B03",
    detail:
      "上传 png/jpg，识别像素图或普通图，再做框定、色板映射与抖动。整条浏览器转图管线归 WP-B03，B02 只留入口。",
  },
  {
    key: "gallery",
    label: "从画廊选图纸",
    owner: null,
    detail: "画廊图纸在灵感里挑，打开详情后「加入待拼」或「转入工作台」即可建成项目。",
    action: { label: "去灵感", to: "/explore" },
  },
  {
    key: "blank",
    label: "空白项目",
    owner: "WP-B06",
    detail: "空白项目要有像素编辑器才有意义（对称、油漆桶、拾色器、色号替换）。编辑器归 WP-B06。",
  },
  {
    key: "import-pattern",
    label: "导入已有豆图",
    owner: "WP-B07",
    detail:
      "导入 .pat / .gamedev / 图片豆图并解析网格。解析与 UNSUPPORTED_FORMAT 失败态归 WP-B07。",
  },
  {
    key: "import-project",
    label: "导入项目库",
    owner: "WP-B07",
    detail: "导入 .beadproj 项目包（schema 落在 src/schema）。导入导出归 WP-B07。",
  },
];

export function CreatePage() {
  useDocumentTitle("创作与导入");
  const [params] = useSearchParams();
  const selectedKey = params.get("entry");
  const selected = ENTRIES.find((entry) => entry.key === selectedKey);

  return (
    <>
      <h1>创作与导入</h1>
      <ul className="project-list">
        {ENTRIES.map((entry) => (
          <li key={entry.key}>
            <Card>
              <Link to={`/create?entry=${entry.key}`}>{entry.label}</Link>
              <p className="stub-note">
                {entry.owner === null ? "现在就能用" : `尚未实现，归 ${entry.owner}`}
              </p>
            </Card>
          </li>
        ))}
      </ul>

      {selected && (
        <section className="section" aria-label={`${selected.label} 说明`}>
          <Card>
            <h2 className="section__title">{selected.label}</h2>
            <p>{selected.detail}</p>
            {selected.owner !== null && (
              <p className="stub-note">
                这是 {selected.owner} 的占位说明页，不是缺陷：功能尚未实现，请勿提 bug。
              </p>
            )}
            {selected.action && (
              <Link className="button button--primary" to={selected.action.to}>
                {selected.action.label}
              </Link>
            )}
          </Card>
        </section>
      )}
    </>
  );
}
