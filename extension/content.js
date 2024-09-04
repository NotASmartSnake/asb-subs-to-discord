function observeMutation(mutations) {
  for (const mutation of mutations) {
    for (const node of mutation.addedNodes) {
      console.log(node.textContent);
      chrome.runtime.sendMessage({ type: "subtitles", text: node.textContent });
    }
  }
}

function findAsb() {
  const container = document.querySelector(".asbplayer-subtitles-container-bottom");
  if (container) {
    let mutationObserver = new MutationObserver(observeMutation);
    mutationObserver.observe(container, {
      childList: true,
    });
    window.top.postMessage("subs found", "*");
  }
}

function unbind(bindings) {
  for (const w of bindings) {
    w.postMessage("unbind", "*");
  }
}

function rootListening() {
  return new Promise((resolve) => {
    let intervalId = setInterval(function() {
      window.top.postMessage("ping", "*");
    }, 500);

    window.addEventListener("message", function(event) {
      if (event.data === "pong") {
        clearInterval(intervalId);
        resolve();
      }
    });
  });
}

async function main() {
  const isRoot = window.self === window.top;

  if (!isRoot) {
    await rootListening();
    window.top.postMessage("bind", "*");
  }

  let intervalId = setInterval(findAsb, 1000);

  if (isRoot) {
    let asbFound = false;
    bindings = []
    window.addEventListener("message", function(event) {
      if (event.data === "subs found") {
        asbFound = true
        clearInterval(intervalId);
        unbind(bindings);
      }
      if (event.data === "bind") {
        if (!asbFound) {
          bindings.push(event.source);
        }
      }
      if (event.data === "ping") {
        event.source.postMessage("pong", "*");
      }
    });
  }

  window.addEventListener("message", function(event) {
    if (event.data === "unbind") {
      clearInterval(intervalId);
    }
  });
}

if (document.readyState === "complete") {
  main();
} else {
  document.addEventListener("readystatechange", main());
}
