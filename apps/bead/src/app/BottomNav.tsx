import { NavLink } from "react-router";

/**
 * Four labelled links with text always visible. 「+」 is a link to a real route
 * (D-UI-3) and carries a readable name rather than a bare glyph; the glyph
 * itself is decorative so screen readers hear 「创作与导入」.
 */
const ITEMS = [
  { to: "/explore", label: "灵感", glyph: null },
  { to: "/workspace", label: "拼装台", glyph: null },
  { to: "/create", label: "创作与导入", glyph: "+" },
  { to: "/inventory", label: "资产", glyph: null },
] as const;

export function BottomNav() {
  return (
    <nav className="bottom-nav" aria-label="主导航">
      {ITEMS.map((item) => (
        <NavLink key={item.to} to={item.to} className="bottom-nav__link">
          {item.glyph !== null && <span aria-hidden="true">{item.glyph}</span>}
          <span>{item.label}</span>
        </NavLink>
      ))}
    </nav>
  );
}
