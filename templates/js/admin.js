(function () {
  var t = document.getElementById("theme-toggle"),
    h = document.documentElement,
    s = localStorage.getItem("theme");
  if (s) h.classList.add("theme-" + s);
  if (!t) return;
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
  var isMac =
    (navigator.userAgentData && navigator.userAgentData.platform === "macOS") ||
    /Mac/.test(navigator.userAgent);
  document.addEventListener("keydown", function (e) {
    var m = isMac ? e.metaKey : e.ctrlKey;
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
})();
