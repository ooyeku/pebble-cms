(function () {
  // ========================================================================
  // Theme toggle
  // ========================================================================
  var t = document.getElementById("theme-toggle"),
    h = document.documentElement,
    s = localStorage.getItem("theme");
  if (s) h.classList.add("theme-" + s);
  if (t) {
    t.addEventListener("click", function () {
      var d =
        h.classList.contains("theme-dark") ||
        (!h.classList.contains("theme-light") &&
          window.matchMedia("(prefers-color-scheme:dark)").matches);
      h.classList.remove("theme-dark", "theme-light");
      if (d) {
        h.classList.add("theme-light");
        localStorage.setItem("theme", "light");
      } else {
        h.classList.add("theme-dark");
        localStorage.setItem("theme", "dark");
      }
    });
  }

  var isMac =
    (navigator.userAgentData && navigator.userAgentData.platform === "macOS") ||
    /Mac/.test(navigator.userAgent);
  var mod = isMac ? "metaKey" : "ctrlKey";

  // ========================================================================
  // Keyboard shortcuts (global)
  // ========================================================================
  document.addEventListener("keydown", function (e) {
    var m = e[mod];
    if (m && e.key === "s") {
      e.preventDefault();
      var f = document.querySelector("form");
      if (f) f.submit();
    }
    if (m && e.shiftKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      var ss = document.getElementById("status"),
        f = document.querySelector("form");
      if (ss && f) {
        ss.value = "published";
        f.submit();
      }
    }
  });

  // ========================================================================
  // Markdown editor enhancements
  // ========================================================================
  var editor = document.getElementById("body_markdown");
  if (!editor) return;

  // --- Formatting shortcuts (Ctrl/Cmd + B/I/K) ---
  function wrapSelection(before, after) {
    var start = editor.selectionStart;
    var end = editor.selectionEnd;
    var text = editor.value;
    var selected = text.substring(start, end);
    editor.value = text.substring(0, start) + before + selected + after + text.substring(end);
    editor.selectionStart = start + before.length;
    editor.selectionEnd = end + before.length;
    editor.focus();
    editor.dispatchEvent(new Event("input", { bubbles: true }));
  }

  function insertAtCursor(text) {
    var start = editor.selectionStart;
    var val = editor.value;
    editor.value = val.substring(0, start) + text + val.substring(start);
    editor.selectionStart = editor.selectionEnd = start + text.length;
    editor.focus();
    editor.dispatchEvent(new Event("input", { bubbles: true }));
  }

  editor.addEventListener("keydown", function (e) {
    var m = e[mod];
    if (m && e.key === "b") {
      e.preventDefault();
      wrapSelection("**", "**");
    }
    if (m && e.key === "i") {
      e.preventDefault();
      wrapSelection("*", "*");
    }
    if (m && e.key === "k") {
      e.preventDefault();
      wrapSelection("[", "](url)");
    }
  });

  // --- Toolbar ---
  var toolbar = document.createElement("div");
  toolbar.className = "editor-toolbar";
  toolbar.innerHTML =
    '<button type="button" data-action="bold" title="Bold (Ctrl+B)"><b>B</b></button>' +
    '<button type="button" data-action="italic" title="Italic (Ctrl+I)"><i>I</i></button>' +
    '<button type="button" data-action="link" title="Link (Ctrl+K)">Link</button>' +
    '<button type="button" data-action="heading" title="Heading">H</button>' +
    '<button type="button" data-action="code" title="Inline Code">&lt;/&gt;</button>' +
    '<button type="button" data-action="quote" title="Blockquote">&gt;</button>' +
    '<button type="button" data-action="ul" title="Bullet List">-</button>' +
    '<button type="button" data-action="ol" title="Numbered List">1.</button>' +
    '<button type="button" data-action="image" title="Image">Img</button>';
  // Insert toolbar before the editor-layout container (not inside it)
  var editorLayout = editor.closest(".editor-layout");
  if (editorLayout) {
    editorLayout.parentNode.insertBefore(toolbar, editorLayout);
  } else {
    editor.parentNode.insertBefore(toolbar, editor);
  }

  toolbar.addEventListener("click", function (e) {
    var btn = e.target.closest("button");
    if (!btn) return;
    var action = btn.getAttribute("data-action");
    switch (action) {
      case "bold": wrapSelection("**", "**"); break;
      case "italic": wrapSelection("*", "*"); break;
      case "link": wrapSelection("[", "](url)"); break;
      case "heading": insertAtCursor("\n## "); break;
      case "code": wrapSelection("`", "`"); break;
      case "quote": insertAtCursor("\n> "); break;
      case "ul": insertAtCursor("\n- "); break;
      case "ol": insertAtCursor("\n1. "); break;
      case "image": insertAtCursor("\n![alt text](image-url)\n"); break;
    }
  });

  // --- Auto-save drafts to localStorage ---
  var formEl = document.querySelector("form");
  var autoSaveKey = "pebble_draft_" + window.location.pathname;
  var autoSaveIndicator = document.createElement("span");
  autoSaveIndicator.className = "autosave-indicator";
  autoSaveIndicator.style.cssText = "font-size:0.75rem;color:var(--text-muted);margin-left:auto;";

  // Insert indicator near the save button
  var btnRow = formEl ? formEl.querySelector("[style*='display: flex']") : null;
  if (btnRow) btnRow.appendChild(autoSaveIndicator);

  // Restore draft
  var savedDraft = localStorage.getItem(autoSaveKey);
  if (savedDraft && editor.value === "") {
    try {
      var draft = JSON.parse(savedDraft);
      if (draft.body && confirm("Restore unsaved draft from " + new Date(draft.ts).toLocaleString() + "?")) {
        editor.value = draft.body;
        var titleEl = document.getElementById("title");
        if (draft.title && titleEl && titleEl.value === "") titleEl.value = draft.title;
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      }
    } catch (e) { /* ignore parse errors */ }
  }

  // Save draft on input
  var autoSaveTimer = null;
  editor.addEventListener("input", function () {
    clearTimeout(autoSaveTimer);
    autoSaveTimer = setTimeout(function () {
      var titleEl = document.getElementById("title");
      localStorage.setItem(autoSaveKey, JSON.stringify({
        body: editor.value,
        title: titleEl ? titleEl.value : "",
        ts: Date.now()
      }));
      autoSaveIndicator.textContent = "Draft saved";
      setTimeout(function () { autoSaveIndicator.textContent = ""; }, 2000);
    }, 2000);
  });

  // Clear draft on successful form submit
  if (formEl) {
    formEl.addEventListener("submit", function () {
      localStorage.removeItem(autoSaveKey);
    });
  }

  // --- Drag-and-drop image upload ---
  editor.addEventListener("dragover", function (e) {
    e.preventDefault();
    editor.classList.add("drag-over");
  });
  editor.addEventListener("dragleave", function () {
    editor.classList.remove("drag-over");
  });
  editor.addEventListener("drop", function (e) {
    e.preventDefault();
    editor.classList.remove("drag-over");
    var files = e.dataTransfer.files;
    if (files.length > 0) uploadAndInsert(files);
  });

  // --- Paste-to-upload (clipboard images) ---
  editor.addEventListener("paste", function (e) {
    var items = e.clipboardData && e.clipboardData.items;
    if (!items) return;
    for (var i = 0; i < items.length; i++) {
      if (items[i].type.indexOf("image") !== -1) {
        e.preventDefault();
        var file = items[i].getAsFile();
        if (file) uploadAndInsert([file]);
        return;
      }
    }
  });

  function uploadAndInsert(files) {
    var formData = new FormData();
    for (var i = 0; i < files.length; i++) {
      formData.append("file", files[i]);
    }
    insertAtCursor("\n![Uploading...]()\n");
    var placeholder = "![Uploading...]()";

    fetch("/admin/media", { method: "POST", body: formData, redirect: "manual" })
      .then(function () {
        // After upload, we need the filename. The redirect goes to /admin/media.
        // We fetch the latest media to get the filename.
        return fetch("/admin/media");
      })
      .then(function (res) { return res.text(); })
      .then(function (html) {
        // Extract the most recent media filename from the response
        var match = html.match(/\/media\/([^"']+\.(?:webp|png|jpg|jpeg|gif|svg))/i);
        if (match) {
          var filename = match[1];
          editor.value = editor.value.replace(placeholder, "![image](/media/" + filename + ")");
        } else {
          editor.value = editor.value.replace(placeholder, "![image uploaded — refresh media page]()");
        }
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      })
      .catch(function () {
        editor.value = editor.value.replace(placeholder, "![upload failed]()");
        editor.dispatchEvent(new Event("input", { bubbles: true }));
      });
  }
})();
