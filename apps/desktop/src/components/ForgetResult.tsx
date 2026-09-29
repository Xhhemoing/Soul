/** Render the core's observations. Do not infer cleanup from checkpoint counts. */
import type { ForgetReceipt } from "../core";
import { cleanupStatus, forgetStatus, type ForgetStatus } from "../forgetOutcome";

const TITLES: Record<ForgetStatus, string> = {
  complete: "遗忘已提交，清理与审计均已确认",
  cleanup_pending: "遗忘已提交，日志清理尚未完成",
  audit_unconfirmed: "遗忘已提交，审计尚未确认",
  cleanup_and_audit_unconfirmed: "遗忘已提交，清理与审计仍待确认",
  preview_mismatch: "遗忘已提交，但影响面与预览不一致",
  unconfirmed: "遗忘结果尚未确认",
};

export function ForgetResult({ receipt }: { readonly receipt: ForgetReceipt }): React.JSX.Element {
  const state = forgetStatus(receipt);
  const cleanup = cleanupStatus(receipt.cleanup);
  return (
    <section className="panel" aria-labelledby="receipt-heading" role="status">
      <h2 id="receipt-heading">{TITLES[state]}</h2>
      <p data-testid="forget-status">{TITLES[state]}</p>
      <ul className="facts" data-testid="forget-receipt">
        <li>回执记录：销毁了 {receipt.content_keys_destroyed} 把内容密钥，
          涉及 {receipt.sealed_blobs_destroyed} 块密封正文。</li>
        <li>回执记录：{receipt.inferences_orphaned} 条推断失去了依据。</li>
        <li data-testid="receipt-matched">
          {receipt.matched_preview
            ? "回执中的数和你看过的那份预览一致。"
            : "回执中的数和预览对不上，请把这件事报告出来。"}
        </li>
        <li data-testid="forget-commit">逻辑销毁：{receipt.logical_committed === true ? "已提交" : "未确认"}。</li>
        <li data-testid="forget-cleanup">日志清理：{cleanup === "complete" ? "已确认"
          : cleanup === "pending" ? "待完成" : "未确认"}。</li>
        <li data-testid="forget-audit">审计写入：{receipt.audit === "recorded" ? "已确认" : "未确认"}。</li>
      </ul>
      {cleanup === "complete" ? null : (
        <p data-testid="forget-wal-warning">旧日志可能仍持有包裹密钥，尚不能宣称清理完成。
          请仅重试日志清理，不要再次确认销毁。</p>
      )}
      {receipt.audit === "recorded" ? null : (
        <p data-testid="forget-audit-warning">未确认不等于未写入；请到审计页核对，不自动补写。</p>
      )}
      {state === "unconfirmed" ? (
        <p>上方数字是收到的回执内容，不是完整成功证明。缺失的确认不会被默认补成成功。</p>
      ) : null}
      <p className="muted">这是逻辑销毁与日志清理结果，不承诺 SSD 磁盘块的物理擦除。</p>
    </section>
  );
}
