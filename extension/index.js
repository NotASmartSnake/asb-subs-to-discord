let input = document.getElementById("uri");

document.getElementById("f").addEventListener("submit", function() {
  chrome.runtime.sendMessage({ type: "setWebsocket", uri: input.value })
});
