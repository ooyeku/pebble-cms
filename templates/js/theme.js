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
})();
