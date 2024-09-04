let webSocket = null

chrome.runtime.onMessage.addListener(function(message, _sender, _sendResponse) {
  if (message.type === "subtitles") {
    if (webSocket) {
      webSocket.send(message.text);
    }
  }
  if (message.type === "setWebsocket") {
    let uri = message.uri
    try {
      let socket = new WebSocket(uri);
      socket.onopen = () => {
        webSocket = socket;
      }
    } catch (e) { }
  }
});
