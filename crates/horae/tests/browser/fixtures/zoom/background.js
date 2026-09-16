// The harness calls chrome.tabs.setZoom through this isolated worker. It never
// loads in a user profile and has no content scripts or external connections.
chrome.runtime.onInstalled.addListener(() => {});
