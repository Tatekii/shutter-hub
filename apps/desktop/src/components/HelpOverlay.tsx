export function HelpOverlay({ onClose }: { onClose: () => void }) {
  const shortcuts: Array<[string, string]> = [
    ["← / →", "切换照片"],
    ["1 – 5", "评分 1-5 星"],
    ["0", "清除评分"],
    ["6 / 7 / 8 / 9", "色标：红 / 黄 / 绿 / 蓝"],
    ["P", "旗标：留存"],
    ["X", "旗标：淘汰"],
    ["U", "清除旗标"],
    ["⌘/Ctrl + 点击", "多选照片"],
    ["⌘/Ctrl + A", "全选当前列表"],
    ["Esc", "关闭面板 / 清除选择"],
    ["?", "显示本帮助"],
  ];

  return (
    <div className="help-overlay" onClick={onClose}>
      <div className="help-dialog" onClick={(e) => e.stopPropagation()}>
        <h2>键盘快捷键</h2>
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
            知道了
          </button>
        </div>
      </div>
    </div>
  );
}
