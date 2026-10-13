import { useT } from "../lib/i18n";

export function HelpOverlay({ onClose }: { onClose: () => void }) {
  const t = useT();
  const shortcuts: Array<[string, string]> = [
    ["← / →", t("help.prevNext")],
    ["双击 / Enter", t("help.openViewer")],
    ["1 – 5", t("help.rate")],
    ["0", t("help.clearRate")],
    ["6 / 7 / 8 / 9", t("help.color")],
    ["P", t("help.pick")],
    ["X", t("help.reject")],
    ["U", t("help.clearFlag")],
    ["⌘/Ctrl + 点击", t("help.multiSelect")],
    ["⌘/Ctrl + A", t("help.selectAll")],
    ["看图内 1-5 / P / X", t("help.autoAdvance")],
    ["看图内 Z", t("help.zoomToggle")],
    ["看图内 D", t("help.develop")],
    ["看图内 滚轮 / 拖拽", t("help.wheelDrag")],
    ["Esc", t("help.escape")],
    ["?", t("help.help")],
  ];

  return (
    <div className="help-overlay" onClick={onClose}>
      <div
        className="help-dialog"
        role="dialog"
        aria-modal="true"
        aria-label={t("help.title")}
        onClick={(e) => e.stopPropagation()}
      >
        <h2>{t("help.title")}</h2>
        <table className="help-table">
          <tbody>
            {shortcuts.map(([keys, desc]) => (
              <tr key={keys}>
                <td className="help-keys">
                  <kbd>{keys}</kbd>
                </td>
                <td>{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="help-footer">
          <button className="btn btn-primary" onClick={onClose}>
            {t("help.done")}
          </button>
        </div>
      </div>
    </div>
  );
}
