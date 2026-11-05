const messages = {
  zh: {
    'Detected drives': '检测到的磁盘', 'Mounted': '已挂载',
    'Scanning…': '正在扫描…', 'Nothing mounted yet.': '尚未挂载磁盘。',
    'Read-only': '只读', 'Mount': '挂载', 'Open': '打开', 'Unmount': '卸载',
    'Ready.': '就绪。', 'Refresh drives': '刷新磁盘', 'Refresh': '刷新',
    'v0.1 · Read-only access': 'v0.1 · 只读访问',
    'Unmount before disconnecting.': '断开设备前请先卸载。',
    'Cancel': '取消', 'Confirm': '确认', 'Snapshot': '快照',
    'No APFS drives detected.': '未检测到 APFS 磁盘。',
  },
};
let language = localStorage.getItem('openapfs-language') || (navigator.language.startsWith('zh') ? 'zh' : 'en');
function t(key) { return (messages[language] || {})[key] || key; }
function applyLanguage() {
  document.documentElement.lang = language === 'zh' ? 'zh-CN' : 'en';
  for (const el of document.querySelectorAll('[data-i18n]')) el.textContent = t(el.dataset.i18n);
}
document.addEventListener('DOMContentLoaded', () => {
  const select = document.getElementById('language');
  select.value = language;
  select.addEventListener('change', () => {
    localStorage.setItem('openapfs-language', select.value);
    location.reload();
  });
  applyLanguage();
});
