// Flame Fine-Grained Reactive Web Runtime (Auto-Generated)
"use strict";

const _signals = new Map();
const _subscribers = new Map();

function _createSignal(name, initialValue) {
  _signals.set(name, initialValue);
  _subscribers.set(name, new Set());
}

function _getSignal(name) {
  return _signals.get(name);
}

function _setSignal(name, nextValue) {
  if (typeof nextValue === 'function') {
    nextValue = nextValue(_signals.get(name));
  }
  _signals.set(name, nextValue);
  const subs = _subscribers.get(name);
  if (subs) {
    for (const sub of subs) {
      sub(nextValue);
    }
  }
}

function _subscribe(name, callback) {
  const subs = _subscribers.get(name);
  if (subs) {
    subs.add(callback);
  }
}

function navigate(url) {
  window.history.pushState({}, "", url);
  _renderActiveRoute();
}
window.navigate = navigate;
window.addEventListener("popstate", () => _renderActiveRoute());

document.addEventListener("click", (e) => {
  const link = e.target.closest("a");
  if (link) {
    const href = link.getAttribute("href");
    if (href && href.startsWith("/") && !link.getAttribute("target") && !link.hasAttribute("download")) {
      e.preventDefault();
      navigate(href);
    }
  }
});

const http = {
  get: async (url) => {
    const res = await fetch(url);
    const textData = await res.text();
    let jsonData = null;
    try {
      jsonData = JSON.parse(textData);
    } catch (_) {}
    const resultData = jsonData !== null ? jsonData : textData;
    return {
      status: res.status,
      ok: res.ok,
      data: resultData,
      text: () => textData,
      json: () => jsonData,
      then: (resolve, reject) => Promise.resolve(resultData).then(resolve, reject),
    };
  },
  post: async (url, body) => {
    const res = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const textData = await res.text();
    let jsonData = null;
    try {
      jsonData = JSON.parse(textData);
    } catch (_) {}
    const resultData = jsonData !== null ? jsonData : textData;
    return {
      status: res.status,
      ok: res.ok,
      data: resultData,
      text: () => textData,
      json: () => jsonData,
      then: (resolve, reject) => Promise.resolve(resultData).then(resolve, reject),
    };
  }
};
window.http = http;

const web = {
  document: typeof document !== 'undefined' ? document : null,
  window: typeof window !== 'undefined' ? window : null,
  navigate: navigate,
  http: http,
  fetch: (url, opts) => fetch(url, opts),
};
window.web = web;
function println(...args) { console.log(...args); }
function print(...args) { console.log(...args); }
window.println = println;
window.print = print;

// WebAssembly Runtime Loader
let _wasmExports = {};
let _wasmReady = false;
async function _initWasm() {
  try {
    const res = await fetch("app.wasm");
    if (res.ok) {
      const { instance } = await WebAssembly.instantiateStreaming(res, {});
      _wasmExports = instance.exports;
      _wasmReady = true;
      window.wasm = _wasmExports;
      if (typeof _onWasmReady === 'function') _onWasmReady();
    }
  } catch (e) {
    try {
      const res = await fetch("app.wasm");
      const bytes = await res.arrayBuffer();
      const { instance } = await WebAssembly.instantiate(bytes, {});
      _wasmExports = instance.exports;
      _wasmReady = true;
      window.wasm = _wasmExports;
      if (typeof _onWasmReady === 'function') _onWasmReady();
    } catch (err) {
      console.warn("⚡ [WASM] WebAssembly instantiation failed (using JS fallback):", err);
    }
  }
}
_initWasm();

_createSignal("btn_color", "#3b82f6");
_createSignal("compute_num", 10);
_createSignal("wasm_speed_badge", "⚡ Native WASM Engine");
_createSignal("count", 0);
_createSignal("contact_status", "Awaiting your message");
_createSignal("wasm_output", "Run WebAssembly computation below");

/* @Computed */
function double_count() {
  return (_getSignal("count") * 2);
}
window.double_count = double_count;

/* @Computed */
function count_parity() {
  if (((_getSignal("count") % 2) === 0)) {
    return "Even Number";
  }
  return "Odd Number";
}
window.count_parity = count_parity;

function toggleColor() {
  if ((_getSignal("btn_color") === "#3b82f6")) {
    _setSignal("btn_color", "#10b981");
  } else {
    if ((_getSignal("btn_color") === "#10b981")) {
      _setSignal("btn_color", "#ec4899");
    } else {
      if ((_getSignal("btn_color") === "#ec4899")) {
        _setSignal("btn_color", "#8b5cf6");
      } else {
        _setSignal("btn_color", "#3b82f6");
      }
    }
  }
}
window.toggleColor = toggleColor;

function setupCursor() {
  web.window.addEventListener("mousemove", (e) => {
    let dot = web.document.getElementById("cursor-dot");
    let outline = web.document.getElementById("cursor-outline");
    if (dot) {
      (dot.style.left = (e.clientX + "px"));
      (dot.style.top = (e.clientY + "px"));
    }
    if (outline) {
      (outline.style.left = (e.clientX + "px"));
      (outline.style.top = (e.clientY + "px"));
    }
});
}
window.setupCursor = setupCursor;
setupCursor();

function _fallback_wasm_fib(n) {
  if ((n <= 1)) {
    return n;
  }
  return (wasm_fib((n - 1)) + wasm_fib((n - 2)));
}
function wasm_fib(n) {
  if (_wasmReady && _wasmExports["wasm_fib"]) {
    return _wasmExports["wasm_fib"](n);
  }
  return _fallback_wasm_fib(n);
}
window.wasm_fib = wasm_fib;

function _fallback_wasm_factorial(n) {
  if ((n <= 1)) {
    return 1;
  }
  return (n * wasm_factorial((n - 1)));
}
function wasm_factorial(n) {
  if (_wasmReady && _wasmExports["wasm_factorial"]) {
    return _wasmExports["wasm_factorial"](n);
  }
  return _fallback_wasm_factorial(n);
}
window.wasm_factorial = wasm_factorial;

function _fallback_wasm_add(a, b) {
  return (a + b);
}
function wasm_add(a, b) {
  if (_wasmReady && _wasmExports["wasm_add"]) {
    return _wasmExports["wasm_add"](a, b);
  }
  return _fallback_wasm_add(a, b);
}
window.wasm_add = wasm_add;

/* @Compute */
function compute_is_prime(n) {
  if ((n <= 1)) {
    return false;
  }
  let d = 2;
  while (((d * d) <= n)) {
    if (((n % d) === 0)) {
      return false;
    }
    (d += 1);
  }
  return true;
}
window.compute_is_prime = compute_is_prime;

/* @Compute */
function compute_collatz_steps(n) {
  let steps = 0;
  let val = n;
  while (((val > 1) && (steps < 1000))) {
    if (((val % 2) === 0)) {
      (val = (val / 2));
    } else {
      (val = ((3 * val) + 1));
    }
    (steps += 1);
  }
  return steps;
}
window.compute_collatz_steps = compute_collatz_steps;

/* @Computed */
function double_compute_num() {
  return (_getSignal("compute_num") * 2);
}
window.double_compute_num = double_compute_num;

/* @Computed */
function square_compute_num() {
  return (_getSignal("compute_num") * _getSignal("compute_num"));
}
window.square_compute_num = square_compute_num;

/* @Computed */
function compute_num_parity() {
  if (((_getSignal("compute_num") % 2) === 0)) {
    return "Even";
  }
  return "Odd";
}
window.compute_num_parity = compute_num_parity;

/* @Computed */
function prime_status() {
  if (compute_is_prime(_getSignal("compute_num"))) {
    return "Yes (Prime)";
  }
  return "No (Composite)";
}
window.prime_status = prime_status;

function on_compute_num_change() {
  web.window.console.log("⚡ [Flame @Effect] compute_num updated: " + (_getSignal("compute_num")));
}
window.on_compute_num_change = on_compute_num_change;

function incrementNum() {
  _setSignal("compute_num", s => s + (1));
}
window.incrementNum = incrementNum;

function decrementNum() {
  if ((_getSignal("compute_num") > 1)) {
    _setSignal("compute_num", s => s - (1));
  }
}
window.decrementNum = decrementNum;

function setPrimeTest() {
  _setSignal("compute_num", 29);
}
window.setPrimeTest = setPrimeTest;

function setCollatzTest() {
  _setSignal("compute_num", 27);
}
window.setCollatzTest = setCollatzTest;

function runWasmFib() {
  let start = web.window.performance.now();
  let res = wasm_fib(_getSignal("compute_num"));
  let end = web.window.performance.now();
  let elapsed = (end - start);
  _setSignal("wasm_output", "Fibonacci(" + (_getSignal("compute_num")) + ") = " + (res));
  _setSignal("wasm_speed_badge", "⚡ WASM executed in " + (elapsed.toFixed(3)) + " ms");
}
window.runWasmFib = runWasmFib;

function runWasmFactorial() {
  let start = web.window.performance.now();
  let res = wasm_factorial(_getSignal("compute_num"));
  let end = web.window.performance.now();
  let elapsed = (end - start);
  _setSignal("wasm_output", "Factorial(" + (_getSignal("compute_num")) + ") = " + (res));
  _setSignal("wasm_speed_badge", "⚡ WASM executed in " + (elapsed.toFixed(3)) + " ms");
}
window.runWasmFactorial = runWasmFactorial;

function submitContact() {
  _setSignal("contact_status", "🚀 Thank you! Your message was sent successfully.");
}
window.submitContact = submitContact;

function getLinkHref(name) {
  if ((name === "Home")) {
    return "/";
  }
  return ("/" + name.toLowerCase());
}
window.getLinkHref = getLinkHref;

async function fetchProjects() {
  let container = web.document.getElementById("projects-grid");
  if (container) {
    (container.innerHTML = "<div class='loading-card'>⚡ Loading live projects via std.net.http...</div>");
  }
  let data = await http.get("https://jsonplaceholder.typicode.com/posts");
  if (container) {
    (container.innerHTML = "");
    let i = 0;
    while (((i < 6) && (i < data.length))) {
      let post = data[i];
      let card = web.document.createElement("div");
      (card.className = "project-card");
      (card.innerHTML = (((((("<span class='project-tag'>Post #" + post.id) + "</span><h3 class='project-title'>") + post.title) + "</h3><p class='project-body'>") + post.body) + "</p>"));
      container.appendChild(card);
      (i += 1);
    }
  }
}
window.fetchProjects = fetchProjects;

// Reactive effects (@Effect)
_subscribe("compute_num", on_compute_num_change);
on_compute_num_change();

function About() {
  const _el0 = document.createElement("div");
  _el0.className = "page-container";
  const _el1 = document.createElement("main");
  _el1.className = "content-section";
  const _el2 = document.createElement("h1");
  _el2.className = "section-title";
  const _t3 = document.createTextNode("About Flame Web");
  _el2.appendChild(_t3);
  _el1.appendChild(_el2);
  const _el4 = document.createElement("p");
  _el4.className = "section-description";
  const _t5 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el4.appendChild(_t5);
  _el1.appendChild(_el4);
  const _el6 = document.createElement("div");
  _el6.className = "features-grid";
  const _el7 = document.createElement("div");
  _el7.className = "feature-card";
  const _el8 = document.createElement("div");
  _el8.className = "feature-icon";
  const _t9 = document.createTextNode("⚡");
  _el8.appendChild(_t9);
  _el7.appendChild(_el8);
  const _el10 = document.createElement("h3");
  const _t11 = document.createTextNode("Fine-Grained Signals");
  _el10.appendChild(_t11);
  _el7.appendChild(_el10);
  const _el12 = document.createElement("p");
  const _t13 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el12.appendChild(_t13);
  _el7.appendChild(_el12);
  _el6.appendChild(_el7);
  const _el14 = document.createElement("div");
  _el14.className = "feature-card";
  const _el15 = document.createElement("div");
  _el15.className = "feature-icon";
  const _t16 = document.createTextNode("🌐");
  _el15.appendChild(_t16);
  _el14.appendChild(_el15);
  const _el17 = document.createElement("h3");
  const _t18 = document.createTextNode("Built-in SPA Router");
  _el17.appendChild(_t18);
  _el14.appendChild(_el17);
  const _el19 = document.createElement("p");
  const _t20 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el19.appendChild(_t20);
  _el14.appendChild(_el19);
  _el6.appendChild(_el14);
  const _el21 = document.createElement("div");
  _el21.className = "feature-card";
  const _el22 = document.createElement("div");
  _el22.className = "feature-icon";
  const _t23 = document.createTextNode("🎨");
  _el22.appendChild(_t23);
  _el21.appendChild(_el22);
  const _el24 = document.createElement("h3");
  const _t25 = document.createTextNode("Scoped & Dynamic Styling");
  _el24.appendChild(_t25);
  _el21.appendChild(_el24);
  const _el26 = document.createElement("p");
  const _t27 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el26.appendChild(_t27);
  _el21.appendChild(_el26);
  _el6.appendChild(_el21);
  _el1.appendChild(_el6);
  _el0.appendChild(_el1);
  return _el0;
}

function compute() {
  const _el28 = document.createElement("div");
  _el28.className = "page-container";
  const _el29 = document.createElement("main");
  _el29.className = "content-section";
  const _el30 = document.createElement("h1");
  _el30.className = "section-title";
  const _t31 = document.createTextNode("⚡ WebAssembly, Compute & Web Annotations");
  _el30.appendChild(_t31);
  _el29.appendChild(_el30);
  const _el32 = document.createElement("p");
  _el32.className = "section-description";
  const _t33 = document.createTextNode("Flame combines fine-grained reactive HTML with high-speed WebAssembly routines and pure mathematical evaluation.");
  _el32.appendChild(_t33);
  _el29.appendChild(_el32);
  const _el34 = document.createElement("div");
  _el34.className = "compute-grid";
  const _el35 = document.createElement("div");
  _el35.className = "compute-card";
  const _el36 = document.createElement("span");
  _el36.className = "badge-wasm";
  const _t37 = document.createTextNode("@Wasm · WebAssembly");
  _el36.appendChild(_t37);
  _el35.appendChild(_el36);
  const _el38 = document.createElement("h2");
  _el38.className = "card-title";
  const _t39 = document.createTextNode("Native WASM VM Engine");
  _el38.appendChild(_t39);
  _el35.appendChild(_el38);
  const _el40 = document.createElement("p");
  _el40.className = "card-desc";
  const _t41 = document.createTextNode("Heavy mathematical routines are compiled directly to WebAssembly binary bytecode and instantiated in the browser.");
  _el40.appendChild(_t41);
  _el35.appendChild(_el40);
  const _el42 = document.createElement("div");
  _el42.className = "compute-val";
  const _t43 = document.createTextNode("");
  const _update__t43 = () => { let _res = _getSignal("wasm_output"); if (typeof _res === 'function') _res = _res(); _t43.textContent = String(_res); };
  _subscribe("wasm_output", _update__t43);
  _update__t43();
  _el42.appendChild(_t43);
  _el35.appendChild(_el42);
  const _el44 = document.createElement("div");
  _el44.className = "stats-row";
  const _el45 = document.createElement("span");
  _el45.className = "badge-wasm";
  const _t46 = document.createTextNode("");
  const _update__t46 = () => { let _res = _getSignal("wasm_speed_badge"); if (typeof _res === 'function') _res = _res(); _t46.textContent = String(_res); };
  _subscribe("wasm_speed_badge", _update__t46);
  _update__t46();
  _el45.appendChild(_t46);
  _el44.appendChild(_el45);
  _el35.appendChild(_el44);
  const _el47 = document.createElement("div");
  _el47.className = "compute-actions";
  const _el48 = document.createElement("button");
  _el48.addEventListener("click", (event) => { runWasmFib(event); });
  _el48.className = "btn-calc";
  const _t49 = document.createTextNode("Run Fib(");
  _el48.appendChild(_t49);
  const _t50 = document.createTextNode("");
  const _update__t50 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t50.textContent = String(_res); };
  _subscribe("compute_num", _update__t50);
  _update__t50();
  _el48.appendChild(_t50);
  const _t51 = document.createTextNode(")");
  _el48.appendChild(_t51);
  _el47.appendChild(_el48);
  const _el52 = document.createElement("button");
  _el52.addEventListener("click", (event) => { runWasmFactorial(event); });
  _el52.className = "btn-calc";
  const _t53 = document.createTextNode("Run Factorial(");
  _el52.appendChild(_t53);
  const _t54 = document.createTextNode("");
  const _update__t54 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t54.textContent = String(_res); };
  _subscribe("compute_num", _update__t54);
  _update__t54();
  _el52.appendChild(_t54);
  const _t55 = document.createTextNode(")");
  _el52.appendChild(_t55);
  _el47.appendChild(_el52);
  _el35.appendChild(_el47);
  _el34.appendChild(_el35);
  const _el56 = document.createElement("div");
  _el56.className = "compute-card";
  const _el57 = document.createElement("span");
  _el57.className = "badge-computed";
  const _t58 = document.createTextNode("@Computed · Reactive Signals");
  _el57.appendChild(_t58);
  _el56.appendChild(_el57);
  const _el59 = document.createElement("h2");
  _el59.className = "card-title";
  const _t60 = document.createTextNode("Reactive Derived Computations");
  _el59.appendChild(_t60);
  _el56.appendChild(_el59);
  const _el61 = document.createElement("p");
  _el61.className = "card-desc";
  const _t62 = document.createTextNode("Automatically tracks dependencies on @State variables and recalculates with zero virtual DOM overhead.");
  _el61.appendChild(_t62);
  _el56.appendChild(_el61);
  const _el63 = document.createElement("div");
  _el63.className = "compute-val";
  const _t64 = document.createTextNode("Active Value:");
  _el63.appendChild(_t64);
  const _t65 = document.createTextNode("");
  const _update__t65 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t65.textContent = String(_res); };
  _subscribe("compute_num", _update__t65);
  _update__t65();
  _el63.appendChild(_t65);
  _el56.appendChild(_el63);
  const _el66 = document.createElement("div");
  _el66.className = "stats-row";
  const _el67 = document.createElement("div");
  _el67.className = "stat-pill";
  const _t68 = document.createTextNode("Double:");
  _el67.appendChild(_t68);
  const _t69 = document.createTextNode("");
  const _update__t69 = () => { let _res = double_compute_num(); if (typeof _res === 'function') _res = _res(); _t69.textContent = String(_res); };
  _subscribe("compute_num", _update__t69);
  _update__t69();
  _el67.appendChild(_t69);
  _el66.appendChild(_el67);
  const _el70 = document.createElement("div");
  _el70.className = "stat-pill";
  const _t71 = document.createTextNode("Squared:");
  _el70.appendChild(_t71);
  const _t72 = document.createTextNode("");
  const _update__t72 = () => { let _res = square_compute_num(); if (typeof _res === 'function') _res = _res(); _t72.textContent = String(_res); };
  _subscribe("compute_num", _update__t72);
  _update__t72();
  _el70.appendChild(_t72);
  _el66.appendChild(_el70);
  const _el73 = document.createElement("div");
  _el73.className = "stat-pill";
  const _t74 = document.createTextNode("Parity:");
  _el73.appendChild(_t74);
  const _t75 = document.createTextNode("");
  const _update__t75 = () => { let _res = compute_num_parity(); if (typeof _res === 'function') _res = _res(); _t75.textContent = String(_res); };
  _subscribe("compute_num", _update__t75);
  _update__t75();
  _el73.appendChild(_t75);
  _el66.appendChild(_el73);
  _el56.appendChild(_el66);
  const _el76 = document.createElement("div");
  _el76.className = "compute-actions";
  const _el77 = document.createElement("button");
  _el77.addEventListener("click", (event) => { incrementNum(event); });
  _el77.className = "btn-calc";
  const _t78 = document.createTextNode("Increment (+1)");
  _el77.appendChild(_t78);
  _el76.appendChild(_el77);
  const _el79 = document.createElement("button");
  _el79.addEventListener("click", (event) => { decrementNum(event); });
  _el79.className = "btn-calc";
  const _t80 = document.createTextNode("Decrement (-1)");
  _el79.appendChild(_t80);
  _el76.appendChild(_el79);
  _el56.appendChild(_el76);
  _el34.appendChild(_el56);
  const _el81 = document.createElement("div");
  _el81.className = "compute-card";
  const _el82 = document.createElement("span");
  _el82.className = "badge-compute";
  const _t83 = document.createTextNode("@Compute · Pure Algorithms");
  _el82.appendChild(_t83);
  _el81.appendChild(_el82);
  const _el84 = document.createElement("h2");
  _el84.className = "card-title";
  const _t85 = document.createTextNode("Optimized Math Routines");
  _el84.appendChild(_t85);
  _el81.appendChild(_el84);
  const _el86 = document.createElement("p");
  _el86.className = "card-desc";
  const _t87 = document.createTextNode("Optimized mathematical calculations executed with maximum efficiency.");
  _el86.appendChild(_t87);
  _el81.appendChild(_el86);
  const _el88 = document.createElement("div");
  _el88.className = "stats-row";
  const _el89 = document.createElement("div");
  _el89.className = "stat-pill";
  const _t90 = document.createTextNode("Prime Test:");
  _el89.appendChild(_t90);
  const _t91 = document.createTextNode("");
  const _update__t91 = () => { let _res = prime_status(); if (typeof _res === 'function') _res = _res(); _t91.textContent = String(_res); };
  _subscribe("compute_num", _update__t91);
  _update__t91();
  _el89.appendChild(_t91);
  _el88.appendChild(_el89);
  _el81.appendChild(_el88);
  const _el92 = document.createElement("div");
  _el92.className = "stats-row";
  const _el93 = document.createElement("div");
  _el93.className = "stat-pill";
  const _t94 = document.createTextNode("Collatz Sequence:");
  _el93.appendChild(_t94);
  const _t95 = document.createTextNode("");
  const _update__t95 = () => { let _res = compute_collatz_steps(_getSignal("compute_num")); if (typeof _res === 'function') _res = _res(); _t95.textContent = String(_res); };
  _subscribe("compute_num", _update__t95);
  _update__t95();
  _el93.appendChild(_t95);
  const _t96 = document.createTextNode("steps");
  _el93.appendChild(_t96);
  _el92.appendChild(_el93);
  _el81.appendChild(_el92);
  const _el97 = document.createElement("div");
  _el97.className = "compute-actions";
  const _el98 = document.createElement("button");
  _el98.addEventListener("click", (event) => { setPrimeTest(event); });
  _el98.className = "btn-calc";
  const _t99 = document.createTextNode("Test Prime (29)");
  _el98.appendChild(_t99);
  _el97.appendChild(_el98);
  const _el100 = document.createElement("button");
  _el100.addEventListener("click", (event) => { setCollatzTest(event); });
  _el100.className = "btn-calc";
  const _t101 = document.createTextNode("Test Collatz (27)");
  _el100.appendChild(_t101);
  _el97.appendChild(_el100);
  _el81.appendChild(_el97);
  _el34.appendChild(_el81);
  _el29.appendChild(_el34);
  _el28.appendChild(_el29);
  return _el28;
}

function index() {
  const _el102 = document.createElement("div");
  _el102.className = "page-container";
  const _el103 = document.createElement("main");
  _el103.className = "hero-section";
  const _el104 = document.createElement("div");
  _el104.className = "hero-badge";
  const _t105 = document.createTextNode("Next-Gen Reactive Framework");
  _el104.appendChild(_t105);
  _el103.appendChild(_el104);
  const _el106 = document.createElement("h1");
  _el106.className = "hero-title";
  const _t107 = document.createTextNode("High-Performance Web in Flame");
  _el106.appendChild(_t107);
  _el103.appendChild(_el106);
  const _el108 = document.createElement("p");
  _el108.className = "hero-subtitle";
  const _t109 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity, WebAssembly (@Wasm), and zero-overhead routing.");
  _el108.appendChild(_t109);
  _el103.appendChild(_el108);
  const _el110 = document.createElement("div");
  _el110.className = "interactive-panel";
  const _el111 = document.createElement("button");
  _el111.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el111.className = "btn-counter";
  const _t112 = document.createTextNode("Counter:");
  _el111.appendChild(_t112);
  const _t113 = document.createTextNode("");
  const _update__t113 = () => { let _res = _getSignal("count"); if (typeof _res === 'function') _res = _res(); _t113.textContent = String(_res); };
  _subscribe("count", _update__t113);
  _update__t113();
  _el111.appendChild(_t113);
  _el110.appendChild(_el111);
  const _el114 = document.createElement("button");
  _el114.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_115 = () => { _el114.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_115);
  _update_attr_115();
  _el114.className = "btn-color-toggle";
  const _t116 = document.createTextNode("Active Color:");
  _el114.appendChild(_t116);
  const _t117 = document.createTextNode("");
  const _update__t117 = () => { let _res = _getSignal("btn_color"); if (typeof _res === 'function') _res = _res(); _t117.textContent = String(_res); };
  _subscribe("btn_color", _update__t117);
  _update__t117();
  _el114.appendChild(_t117);
  _el110.appendChild(_el114);
  _el103.appendChild(_el110);
  const _el118 = document.createElement("div");
  _el118.className = "computed-stats";
  _el118.style.cssText = "margin-top: 1.5rem; display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap;";
  const _el119 = document.createElement("div");
  _el119.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t120 = document.createTextNode("⚡");
  _el119.appendChild(_t120);
  const _el121 = document.createElement("strong");
  const _t122 = document.createTextNode("@Computed Double:");
  _el121.appendChild(_t122);
  _el119.appendChild(_el121);
  const _t123 = document.createTextNode("");
  const _update__t123 = () => { let _res = double_count(); if (typeof _res === 'function') _res = _res(); _t123.textContent = String(_res); };
  _subscribe("count", _update__t123);
  _update__t123();
  _el119.appendChild(_t123);
  _el118.appendChild(_el119);
  const _el124 = document.createElement("div");
  _el124.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t125 = document.createTextNode("🎯");
  _el124.appendChild(_t125);
  const _el126 = document.createElement("strong");
  const _t127 = document.createTextNode("Parity:");
  _el126.appendChild(_t127);
  _el124.appendChild(_el126);
  const _t128 = document.createTextNode("");
  const _update__t128 = () => { let _res = count_parity(); if (typeof _res === 'function') _res = _res(); _t128.textContent = String(_res); };
  _subscribe("count", _update__t128);
  _update__t128();
  _el124.appendChild(_t128);
  _el118.appendChild(_el124);
  _el103.appendChild(_el118);
  _el102.appendChild(_el103);
  return _el102;
}

function Contact() {
  const _el129 = document.createElement("div");
  _el129.className = "page-container";
  const _el130 = document.createElement("main");
  _el130.className = "content-section";
  const _el131 = document.createElement("h1");
  _el131.className = "section-title";
  const _t132 = document.createTextNode("Get in Touch");
  _el131.appendChild(_t132);
  _el130.appendChild(_el131);
  const _el133 = document.createElement("p");
  _el133.className = "section-description";
  const _t134 = document.createTextNode("Send a message through our reactive contact form.");
  _el133.appendChild(_t134);
  _el130.appendChild(_el133);
  const _el135 = document.createElement("div");
  _el135.className = "contact-box";
  const _el136 = document.createElement("div");
  _el136.className = "form-group";
  const _el137 = document.createElement("label");
  const _t138 = document.createTextNode("Your Name");
  _el137.appendChild(_t138);
  _el136.appendChild(_el137);
  const _el139 = document.createElement("input");
  _el139.setAttribute("id", "contact-name");
  _el139.setAttribute("type", "text");
  _el139.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el139.className = "input-field";
  _el136.appendChild(_el139);
  _el135.appendChild(_el136);
  const _el140 = document.createElement("div");
  _el140.className = "form-group";
  const _el141 = document.createElement("label");
  const _t142 = document.createTextNode("Message");
  _el141.appendChild(_t142);
  _el140.appendChild(_el141);
  const _el143 = document.createElement("textarea");
  _el143.setAttribute("id", "contact-msg");
  _el143.setAttribute("placeholder", "Tell us about your project...");
  _el143.className = "input-field textarea";
  _el140.appendChild(_el143);
  _el135.appendChild(_el140);
  const _el144 = document.createElement("button");
  _el144.addEventListener("click", (event) => { submitContact(event); });
  _el144.className = "btn-primary";
  const _t145 = document.createTextNode("Send Message");
  _el144.appendChild(_t145);
  _el135.appendChild(_el144);
  const _el146 = document.createElement("div");
  _el146.className = "status-banner";
  const _t147 = document.createTextNode("Status:");
  _el146.appendChild(_t147);
  const _t148 = document.createTextNode("");
  const _update__t148 = () => { let _res = _getSignal("contact_status"); if (typeof _res === 'function') _res = _res(); _t148.textContent = String(_res); };
  _subscribe("contact_status", _update__t148);
  _update__t148();
  _el146.appendChild(_t148);
  _el135.appendChild(_el146);
  _el130.appendChild(_el135);
  _el129.appendChild(_el130);
  return _el129;
}

function about() {
  const _el149 = document.createElement("div");
  _el149.className = "page-container";
  const _el150 = document.createElement("main");
  _el150.className = "content-section";
  const _el151 = document.createElement("h1");
  _el151.className = "section-title";
  const _t152 = document.createTextNode("About Flame Web");
  _el151.appendChild(_t152);
  _el150.appendChild(_el151);
  const _el153 = document.createElement("p");
  _el153.className = "section-description";
  const _t154 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el153.appendChild(_t154);
  _el150.appendChild(_el153);
  const _el155 = document.createElement("div");
  _el155.className = "features-grid";
  const _el156 = document.createElement("div");
  _el156.className = "feature-card";
  const _el157 = document.createElement("div");
  _el157.className = "feature-icon";
  const _t158 = document.createTextNode("⚡");
  _el157.appendChild(_t158);
  _el156.appendChild(_el157);
  const _el159 = document.createElement("h3");
  const _t160 = document.createTextNode("Fine-Grained Signals");
  _el159.appendChild(_t160);
  _el156.appendChild(_el159);
  const _el161 = document.createElement("p");
  const _t162 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el161.appendChild(_t162);
  _el156.appendChild(_el161);
  _el155.appendChild(_el156);
  const _el163 = document.createElement("div");
  _el163.className = "feature-card";
  const _el164 = document.createElement("div");
  _el164.className = "feature-icon";
  const _t165 = document.createTextNode("🌐");
  _el164.appendChild(_t165);
  _el163.appendChild(_el164);
  const _el166 = document.createElement("h3");
  const _t167 = document.createTextNode("Built-in SPA Router");
  _el166.appendChild(_t167);
  _el163.appendChild(_el166);
  const _el168 = document.createElement("p");
  const _t169 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el168.appendChild(_t169);
  _el163.appendChild(_el168);
  _el155.appendChild(_el163);
  const _el170 = document.createElement("div");
  _el170.className = "feature-card";
  const _el171 = document.createElement("div");
  _el171.className = "feature-icon";
  const _t172 = document.createTextNode("🎨");
  _el171.appendChild(_t172);
  _el170.appendChild(_el171);
  const _el173 = document.createElement("h3");
  const _t174 = document.createTextNode("Scoped & Dynamic Styling");
  _el173.appendChild(_t174);
  _el170.appendChild(_el173);
  const _el175 = document.createElement("p");
  const _t176 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el175.appendChild(_t176);
  _el170.appendChild(_el175);
  _el155.appendChild(_el170);
  _el150.appendChild(_el155);
  _el149.appendChild(_el150);
  return _el149;
}

function Compute() {
  const _el177 = document.createElement("div");
  _el177.className = "page-container";
  const _el178 = document.createElement("main");
  _el178.className = "content-section";
  const _el179 = document.createElement("h1");
  _el179.className = "section-title";
  const _t180 = document.createTextNode("⚡ WebAssembly, Compute & Web Annotations");
  _el179.appendChild(_t180);
  _el178.appendChild(_el179);
  const _el181 = document.createElement("p");
  _el181.className = "section-description";
  const _t182 = document.createTextNode("Flame combines fine-grained reactive HTML with high-speed WebAssembly routines and pure mathematical evaluation.");
  _el181.appendChild(_t182);
  _el178.appendChild(_el181);
  const _el183 = document.createElement("div");
  _el183.className = "compute-grid";
  const _el184 = document.createElement("div");
  _el184.className = "compute-card";
  const _el185 = document.createElement("span");
  _el185.className = "badge-wasm";
  const _t186 = document.createTextNode("@Wasm · WebAssembly");
  _el185.appendChild(_t186);
  _el184.appendChild(_el185);
  const _el187 = document.createElement("h2");
  _el187.className = "card-title";
  const _t188 = document.createTextNode("Native WASM VM Engine");
  _el187.appendChild(_t188);
  _el184.appendChild(_el187);
  const _el189 = document.createElement("p");
  _el189.className = "card-desc";
  const _t190 = document.createTextNode("Heavy mathematical routines are compiled directly to WebAssembly binary bytecode and instantiated in the browser.");
  _el189.appendChild(_t190);
  _el184.appendChild(_el189);
  const _el191 = document.createElement("div");
  _el191.className = "compute-val";
  const _t192 = document.createTextNode("");
  const _update__t192 = () => { let _res = _getSignal("wasm_output"); if (typeof _res === 'function') _res = _res(); _t192.textContent = String(_res); };
  _subscribe("wasm_output", _update__t192);
  _update__t192();
  _el191.appendChild(_t192);
  _el184.appendChild(_el191);
  const _el193 = document.createElement("div");
  _el193.className = "stats-row";
  const _el194 = document.createElement("span");
  _el194.className = "badge-wasm";
  const _t195 = document.createTextNode("");
  const _update__t195 = () => { let _res = _getSignal("wasm_speed_badge"); if (typeof _res === 'function') _res = _res(); _t195.textContent = String(_res); };
  _subscribe("wasm_speed_badge", _update__t195);
  _update__t195();
  _el194.appendChild(_t195);
  _el193.appendChild(_el194);
  _el184.appendChild(_el193);
  const _el196 = document.createElement("div");
  _el196.className = "compute-actions";
  const _el197 = document.createElement("button");
  _el197.addEventListener("click", (event) => { runWasmFib(event); });
  _el197.className = "btn-calc";
  const _t198 = document.createTextNode("Run Fib(");
  _el197.appendChild(_t198);
  const _t199 = document.createTextNode("");
  const _update__t199 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t199.textContent = String(_res); };
  _subscribe("compute_num", _update__t199);
  _update__t199();
  _el197.appendChild(_t199);
  const _t200 = document.createTextNode(")");
  _el197.appendChild(_t200);
  _el196.appendChild(_el197);
  const _el201 = document.createElement("button");
  _el201.addEventListener("click", (event) => { runWasmFactorial(event); });
  _el201.className = "btn-calc";
  const _t202 = document.createTextNode("Run Factorial(");
  _el201.appendChild(_t202);
  const _t203 = document.createTextNode("");
  const _update__t203 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t203.textContent = String(_res); };
  _subscribe("compute_num", _update__t203);
  _update__t203();
  _el201.appendChild(_t203);
  const _t204 = document.createTextNode(")");
  _el201.appendChild(_t204);
  _el196.appendChild(_el201);
  _el184.appendChild(_el196);
  _el183.appendChild(_el184);
  const _el205 = document.createElement("div");
  _el205.className = "compute-card";
  const _el206 = document.createElement("span");
  _el206.className = "badge-computed";
  const _t207 = document.createTextNode("@Computed · Reactive Signals");
  _el206.appendChild(_t207);
  _el205.appendChild(_el206);
  const _el208 = document.createElement("h2");
  _el208.className = "card-title";
  const _t209 = document.createTextNode("Reactive Derived Computations");
  _el208.appendChild(_t209);
  _el205.appendChild(_el208);
  const _el210 = document.createElement("p");
  _el210.className = "card-desc";
  const _t211 = document.createTextNode("Automatically tracks dependencies on @State variables and recalculates with zero virtual DOM overhead.");
  _el210.appendChild(_t211);
  _el205.appendChild(_el210);
  const _el212 = document.createElement("div");
  _el212.className = "compute-val";
  const _t213 = document.createTextNode("Active Value:");
  _el212.appendChild(_t213);
  const _t214 = document.createTextNode("");
  const _update__t214 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t214.textContent = String(_res); };
  _subscribe("compute_num", _update__t214);
  _update__t214();
  _el212.appendChild(_t214);
  _el205.appendChild(_el212);
  const _el215 = document.createElement("div");
  _el215.className = "stats-row";
  const _el216 = document.createElement("div");
  _el216.className = "stat-pill";
  const _t217 = document.createTextNode("Double:");
  _el216.appendChild(_t217);
  const _t218 = document.createTextNode("");
  const _update__t218 = () => { let _res = double_compute_num(); if (typeof _res === 'function') _res = _res(); _t218.textContent = String(_res); };
  _subscribe("compute_num", _update__t218);
  _update__t218();
  _el216.appendChild(_t218);
  _el215.appendChild(_el216);
  const _el219 = document.createElement("div");
  _el219.className = "stat-pill";
  const _t220 = document.createTextNode("Squared:");
  _el219.appendChild(_t220);
  const _t221 = document.createTextNode("");
  const _update__t221 = () => { let _res = square_compute_num(); if (typeof _res === 'function') _res = _res(); _t221.textContent = String(_res); };
  _subscribe("compute_num", _update__t221);
  _update__t221();
  _el219.appendChild(_t221);
  _el215.appendChild(_el219);
  const _el222 = document.createElement("div");
  _el222.className = "stat-pill";
  const _t223 = document.createTextNode("Parity:");
  _el222.appendChild(_t223);
  const _t224 = document.createTextNode("");
  const _update__t224 = () => { let _res = compute_num_parity(); if (typeof _res === 'function') _res = _res(); _t224.textContent = String(_res); };
  _subscribe("compute_num", _update__t224);
  _update__t224();
  _el222.appendChild(_t224);
  _el215.appendChild(_el222);
  _el205.appendChild(_el215);
  const _el225 = document.createElement("div");
  _el225.className = "compute-actions";
  const _el226 = document.createElement("button");
  _el226.addEventListener("click", (event) => { incrementNum(event); });
  _el226.className = "btn-calc";
  const _t227 = document.createTextNode("Increment (+1)");
  _el226.appendChild(_t227);
  _el225.appendChild(_el226);
  const _el228 = document.createElement("button");
  _el228.addEventListener("click", (event) => { decrementNum(event); });
  _el228.className = "btn-calc";
  const _t229 = document.createTextNode("Decrement (-1)");
  _el228.appendChild(_t229);
  _el225.appendChild(_el228);
  _el205.appendChild(_el225);
  _el183.appendChild(_el205);
  const _el230 = document.createElement("div");
  _el230.className = "compute-card";
  const _el231 = document.createElement("span");
  _el231.className = "badge-compute";
  const _t232 = document.createTextNode("@Compute · Pure Algorithms");
  _el231.appendChild(_t232);
  _el230.appendChild(_el231);
  const _el233 = document.createElement("h2");
  _el233.className = "card-title";
  const _t234 = document.createTextNode("Optimized Math Routines");
  _el233.appendChild(_t234);
  _el230.appendChild(_el233);
  const _el235 = document.createElement("p");
  _el235.className = "card-desc";
  const _t236 = document.createTextNode("Optimized mathematical calculations executed with maximum efficiency.");
  _el235.appendChild(_t236);
  _el230.appendChild(_el235);
  const _el237 = document.createElement("div");
  _el237.className = "stats-row";
  const _el238 = document.createElement("div");
  _el238.className = "stat-pill";
  const _t239 = document.createTextNode("Prime Test:");
  _el238.appendChild(_t239);
  const _t240 = document.createTextNode("");
  const _update__t240 = () => { let _res = prime_status(); if (typeof _res === 'function') _res = _res(); _t240.textContent = String(_res); };
  _subscribe("compute_num", _update__t240);
  _update__t240();
  _el238.appendChild(_t240);
  _el237.appendChild(_el238);
  _el230.appendChild(_el237);
  const _el241 = document.createElement("div");
  _el241.className = "stats-row";
  const _el242 = document.createElement("div");
  _el242.className = "stat-pill";
  const _t243 = document.createTextNode("Collatz Sequence:");
  _el242.appendChild(_t243);
  const _t244 = document.createTextNode("");
  const _update__t244 = () => { let _res = compute_collatz_steps(_getSignal("compute_num")); if (typeof _res === 'function') _res = _res(); _t244.textContent = String(_res); };
  _subscribe("compute_num", _update__t244);
  _update__t244();
  _el242.appendChild(_t244);
  const _t245 = document.createTextNode("steps");
  _el242.appendChild(_t245);
  _el241.appendChild(_el242);
  _el230.appendChild(_el241);
  const _el246 = document.createElement("div");
  _el246.className = "compute-actions";
  const _el247 = document.createElement("button");
  _el247.addEventListener("click", (event) => { setPrimeTest(event); });
  _el247.className = "btn-calc";
  const _t248 = document.createTextNode("Test Prime (29)");
  _el247.appendChild(_t248);
  _el246.appendChild(_el247);
  const _el249 = document.createElement("button");
  _el249.addEventListener("click", (event) => { setCollatzTest(event); });
  _el249.className = "btn-calc";
  const _t250 = document.createTextNode("Test Collatz (27)");
  _el249.appendChild(_t250);
  _el246.appendChild(_el249);
  _el230.appendChild(_el246);
  _el183.appendChild(_el230);
  _el178.appendChild(_el183);
  _el177.appendChild(_el178);
  return _el177;
}

function Projects() {
  const _el251 = document.createElement("div");
  _el251.className = "page-container";
  const _el252 = document.createElement("main");
  _el252.className = "content-section";
  const _el253 = document.createElement("h1");
  _el253.className = "section-title";
  const _t254 = document.createTextNode("Live API Projects");
  _el253.appendChild(_t254);
  _el252.appendChild(_el253);
  const _el255 = document.createElement("p");
  _el255.className = "section-description";
  const _t256 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el255.appendChild(_t256);
  _el252.appendChild(_el255);
  const _el257 = document.createElement("div");
  _el257.className = "action-bar";
  const _el258 = document.createElement("button");
  _el258.addEventListener("click", (event) => { fetchProjects(event); });
  _el258.className = "btn-primary";
  const _t259 = document.createTextNode("Fetch Live Posts via http.get()");
  _el258.appendChild(_t259);
  _el257.appendChild(_el258);
  _el252.appendChild(_el257);
  const _el260 = document.createElement("div");
  _el260.setAttribute("id", "projects-grid");
  _el260.className = "projects-container";
  const _el261 = document.createElement("div");
  _el261.className = "empty-state";
  const _t262 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el261.appendChild(_t262);
  _el260.appendChild(_el261);
  _el252.appendChild(_el260);
  _el251.appendChild(_el252);
  return _el251;
}

function contact() {
  const _el263 = document.createElement("div");
  _el263.className = "page-container";
  const _el264 = document.createElement("main");
  _el264.className = "content-section";
  const _el265 = document.createElement("h1");
  _el265.className = "section-title";
  const _t266 = document.createTextNode("Get in Touch");
  _el265.appendChild(_t266);
  _el264.appendChild(_el265);
  const _el267 = document.createElement("p");
  _el267.className = "section-description";
  const _t268 = document.createTextNode("Send a message through our reactive contact form.");
  _el267.appendChild(_t268);
  _el264.appendChild(_el267);
  const _el269 = document.createElement("div");
  _el269.className = "contact-box";
  const _el270 = document.createElement("div");
  _el270.className = "form-group";
  const _el271 = document.createElement("label");
  const _t272 = document.createTextNode("Your Name");
  _el271.appendChild(_t272);
  _el270.appendChild(_el271);
  const _el273 = document.createElement("input");
  _el273.setAttribute("id", "contact-name");
  _el273.setAttribute("type", "text");
  _el273.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el273.className = "input-field";
  _el270.appendChild(_el273);
  _el269.appendChild(_el270);
  const _el274 = document.createElement("div");
  _el274.className = "form-group";
  const _el275 = document.createElement("label");
  const _t276 = document.createTextNode("Message");
  _el275.appendChild(_t276);
  _el274.appendChild(_el275);
  const _el277 = document.createElement("textarea");
  _el277.setAttribute("id", "contact-msg");
  _el277.setAttribute("placeholder", "Tell us about your project...");
  _el277.className = "input-field textarea";
  _el274.appendChild(_el277);
  _el269.appendChild(_el274);
  const _el278 = document.createElement("button");
  _el278.addEventListener("click", (event) => { submitContact(event); });
  _el278.className = "btn-primary";
  const _t279 = document.createTextNode("Send Message");
  _el278.appendChild(_t279);
  _el269.appendChild(_el278);
  const _el280 = document.createElement("div");
  _el280.className = "status-banner";
  const _t281 = document.createTextNode("Status:");
  _el280.appendChild(_t281);
  const _t282 = document.createTextNode("");
  const _update__t282 = () => { let _res = _getSignal("contact_status"); if (typeof _res === 'function') _res = _res(); _t282.textContent = String(_res); };
  _subscribe("contact_status", _update__t282);
  _update__t282();
  _el280.appendChild(_t282);
  _el269.appendChild(_el280);
  _el264.appendChild(_el269);
  _el263.appendChild(_el264);
  return _el263;
}

function projects() {
  const _el283 = document.createElement("div");
  _el283.className = "page-container";
  const _el284 = document.createElement("main");
  _el284.className = "content-section";
  const _el285 = document.createElement("h1");
  _el285.className = "section-title";
  const _t286 = document.createTextNode("Live API Projects");
  _el285.appendChild(_t286);
  _el284.appendChild(_el285);
  const _el287 = document.createElement("p");
  _el287.className = "section-description";
  const _t288 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el287.appendChild(_t288);
  _el284.appendChild(_el287);
  const _el289 = document.createElement("div");
  _el289.className = "action-bar";
  const _el290 = document.createElement("button");
  _el290.addEventListener("click", (event) => { fetchProjects(event); });
  _el290.className = "btn-primary";
  const _t291 = document.createTextNode("Fetch Live Posts via http.get()");
  _el290.appendChild(_t291);
  _el289.appendChild(_el290);
  _el284.appendChild(_el289);
  const _el292 = document.createElement("div");
  _el292.setAttribute("id", "projects-grid");
  _el292.className = "projects-container";
  const _el293 = document.createElement("div");
  _el293.className = "empty-state";
  const _t294 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el293.appendChild(_t294);
  _el292.appendChild(_el293);
  _el284.appendChild(_el292);
  _el283.appendChild(_el284);
  return _el283;
}

function Nav(_props) {
  let elem = (_props && typeof _props === 'object' && !Array.isArray(_props) && !(typeof Node !== 'undefined' && _props instanceof Node) && _props.constructor === Object && _props["elem"] !== undefined) ? _props["elem"] : arguments[0];
  const _el295 = document.createElement("header");
  _el295.className = "navbar";
  const _el296 = document.createElement("div");
  _el296.setAttribute("id", "cursor-dot");
  _el296.className = "cursor-dot";
  _el295.appendChild(_el296);
  const _el297 = document.createElement("div");
  _el297.setAttribute("id", "cursor-outline");
  _el297.className = "cursor-outline";
  _el295.appendChild(_el297);
  const _el298 = document.createElement("div");
  _el298.className = "nav-brand";
  const _el299 = document.createElement("span");
  _el299.className = "brand-icon";
  const _t300 = document.createTextNode("⚡");
  _el299.appendChild(_t300);
  _el298.appendChild(_el299);
  const _el301 = document.createElement("span");
  _el301.className = "brand-text";
  const _t302 = document.createTextNode("Flame Web");
  _el301.appendChild(_t302);
  _el298.appendChild(_el301);
  _el295.appendChild(_el298);
  const _el303 = document.createElement("nav");
  _el303.className = "nav-links";
  const _iter304 = elem;
  if (Array.isArray(_iter304) || (_iter304 && typeof _iter304[Symbol.iterator] === 'function')) {
    for (const e of _iter304) {
      const _el305 = document.createElement("a");
      _el305.setAttribute("href", getLinkHref(e));
      _el305.className = "nav-link";
      let _val307 = e;
      if (typeof _val307 === 'function') _val307 = _val307();
      if (typeof _val307 === 'object' && _val307 instanceof Node) {
        _el305.appendChild(_val307);
      } else if (Array.isArray(_val307) || (_val307 && typeof _val307 !== 'string' && typeof _val307[Symbol.iterator] === 'function')) {
        for (const _item of _val307) {
          if (typeof _item === 'object' && _item instanceof Node) {
            _el305.appendChild(_item);
          } else {
            _el305.appendChild(document.createTextNode(String(_item)));
          }
        }
      } else {
        _el305.appendChild(document.createTextNode(String(_val307)));
      }
      _el303.appendChild(_el305);
    }
  }
  _el295.appendChild(_el303);
  return _el295;
}

function NavBar() {
  const _comp308 = Nav({"elem": ["Home", "About", "Compute", "Projects", "Contact"]});
  return _comp308;
}

function Index() {
  const _el309 = document.createElement("div");
  _el309.className = "page-container";
  const _el310 = document.createElement("main");
  _el310.className = "hero-section";
  const _el311 = document.createElement("div");
  _el311.className = "hero-badge";
  const _t312 = document.createTextNode("Next-Gen Reactive Framework");
  _el311.appendChild(_t312);
  _el310.appendChild(_el311);
  const _el313 = document.createElement("h1");
  _el313.className = "hero-title";
  const _t314 = document.createTextNode("High-Performance Web in Flame");
  _el313.appendChild(_t314);
  _el310.appendChild(_el313);
  const _el315 = document.createElement("p");
  _el315.className = "hero-subtitle";
  const _t316 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity, WebAssembly (@Wasm), and zero-overhead routing.");
  _el315.appendChild(_t316);
  _el310.appendChild(_el315);
  const _el317 = document.createElement("div");
  _el317.className = "interactive-panel";
  const _el318 = document.createElement("button");
  _el318.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el318.className = "btn-counter";
  const _t319 = document.createTextNode("Counter:");
  _el318.appendChild(_t319);
  const _t320 = document.createTextNode("");
  const _update__t320 = () => { let _res = _getSignal("count"); if (typeof _res === 'function') _res = _res(); _t320.textContent = String(_res); };
  _subscribe("count", _update__t320);
  _update__t320();
  _el318.appendChild(_t320);
  _el317.appendChild(_el318);
  const _el321 = document.createElement("button");
  _el321.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_322 = () => { _el321.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_322);
  _update_attr_322();
  _el321.className = "btn-color-toggle";
  const _t323 = document.createTextNode("Active Color:");
  _el321.appendChild(_t323);
  const _t324 = document.createTextNode("");
  const _update__t324 = () => { let _res = _getSignal("btn_color"); if (typeof _res === 'function') _res = _res(); _t324.textContent = String(_res); };
  _subscribe("btn_color", _update__t324);
  _update__t324();
  _el321.appendChild(_t324);
  _el317.appendChild(_el321);
  _el310.appendChild(_el317);
  const _el325 = document.createElement("div");
  _el325.className = "computed-stats";
  _el325.style.cssText = "margin-top: 1.5rem; display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap;";
  const _el326 = document.createElement("div");
  _el326.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t327 = document.createTextNode("⚡");
  _el326.appendChild(_t327);
  const _el328 = document.createElement("strong");
  const _t329 = document.createTextNode("@Computed Double:");
  _el328.appendChild(_t329);
  _el326.appendChild(_el328);
  const _t330 = document.createTextNode("");
  const _update__t330 = () => { let _res = double_count(); if (typeof _res === 'function') _res = _res(); _t330.textContent = String(_res); };
  _subscribe("count", _update__t330);
  _update__t330();
  _el326.appendChild(_t330);
  _el325.appendChild(_el326);
  const _el331 = document.createElement("div");
  _el331.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t332 = document.createTextNode("🎯");
  _el331.appendChild(_t332);
  const _el333 = document.createElement("strong");
  const _t334 = document.createTextNode("Parity:");
  _el333.appendChild(_t334);
  _el331.appendChild(_el333);
  const _t335 = document.createTextNode("");
  const _update__t335 = () => { let _res = count_parity(); if (typeof _res === 'function') _res = _res(); _t335.textContent = String(_res); };
  _subscribe("count", _update__t335);
  _update__t335();
  _el331.appendChild(_t335);
  _el325.appendChild(_el331);
  _el310.appendChild(_el325);
  _el309.appendChild(_el310);
  return _el309;
}

function AppLayout(_props) {
  let children = (_props && typeof _props === 'object' && !Array.isArray(_props) && !(typeof Node !== 'undefined' && _props instanceof Node) && _props.constructor === Object && _props["children"] !== undefined) ? _props["children"] : arguments[0];
  const _el336 = document.createElement("div");
  _el336.className = "app-layout";
  const _comp337 = NavBar({});
  _el336.appendChild(_comp337);
  const _el338 = document.createElement("div");
  _el338.className = "app-content";
  let _val340 = children;
  if (typeof _val340 === 'function') _val340 = _val340();
  if (typeof _val340 === 'object' && _val340 instanceof Node) {
    _el338.appendChild(_val340);
  } else if (Array.isArray(_val340) || (_val340 && typeof _val340 !== 'string' && typeof _val340[Symbol.iterator] === 'function')) {
    for (const _item of _val340) {
      if (typeof _item === 'object' && _item instanceof Node) {
        _el338.appendChild(_item);
      } else {
        _el338.appendChild(document.createTextNode(String(_item)));
      }
    }
  } else {
    _el338.appendChild(document.createTextNode(String(_val340)));
  }
  _el336.appendChild(_el338);
  return _el336;
}

function _render_page_index() {
  document.title = "Flame Web Showcase";
  const _el341 = document.createElement("div");
  _el341.className = "page-container";
  const _el342 = document.createElement("main");
  _el342.className = "hero-section";
  const _el343 = document.createElement("div");
  _el343.className = "hero-badge";
  const _t344 = document.createTextNode("Next-Gen Reactive Framework");
  _el343.appendChild(_t344);
  _el342.appendChild(_el343);
  const _el345 = document.createElement("h1");
  _el345.className = "hero-title";
  const _t346 = document.createTextNode("High-Performance Web in Flame");
  _el345.appendChild(_t346);
  _el342.appendChild(_el345);
  const _el347 = document.createElement("p");
  _el347.className = "hero-subtitle";
  const _t348 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity, WebAssembly (@Wasm), and zero-overhead routing.");
  _el347.appendChild(_t348);
  _el342.appendChild(_el347);
  const _el349 = document.createElement("div");
  _el349.className = "interactive-panel";
  const _el350 = document.createElement("button");
  _el350.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el350.className = "btn-counter";
  const _t351 = document.createTextNode("Counter:");
  _el350.appendChild(_t351);
  const _t352 = document.createTextNode("");
  const _update__t352 = () => { let _res = _getSignal("count"); if (typeof _res === 'function') _res = _res(); _t352.textContent = String(_res); };
  _subscribe("count", _update__t352);
  _update__t352();
  _el350.appendChild(_t352);
  _el349.appendChild(_el350);
  const _el353 = document.createElement("button");
  _el353.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_354 = () => { _el353.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_354);
  _update_attr_354();
  _el353.className = "btn-color-toggle";
  const _t355 = document.createTextNode("Active Color:");
  _el353.appendChild(_t355);
  const _t356 = document.createTextNode("");
  const _update__t356 = () => { let _res = _getSignal("btn_color"); if (typeof _res === 'function') _res = _res(); _t356.textContent = String(_res); };
  _subscribe("btn_color", _update__t356);
  _update__t356();
  _el353.appendChild(_t356);
  _el349.appendChild(_el353);
  _el342.appendChild(_el349);
  const _el357 = document.createElement("div");
  _el357.className = "computed-stats";
  _el357.style.cssText = "margin-top: 1.5rem; display: flex; gap: 1rem; justify-content: center; flex-wrap: wrap;";
  const _el358 = document.createElement("div");
  _el358.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t359 = document.createTextNode("⚡");
  _el358.appendChild(_t359);
  const _el360 = document.createElement("strong");
  const _t361 = document.createTextNode("@Computed Double:");
  _el360.appendChild(_t361);
  _el358.appendChild(_el360);
  const _t362 = document.createTextNode("");
  const _update__t362 = () => { let _res = double_count(); if (typeof _res === 'function') _res = _res(); _t362.textContent = String(_res); };
  _subscribe("count", _update__t362);
  _update__t362();
  _el358.appendChild(_t362);
  _el357.appendChild(_el358);
  const _el363 = document.createElement("div");
  _el363.style.cssText = "background: rgba(255, 255, 255, 0.05); padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid rgba(255, 255, 255, 0.1);";
  const _t364 = document.createTextNode("🎯");
  _el363.appendChild(_t364);
  const _el365 = document.createElement("strong");
  const _t366 = document.createTextNode("Parity:");
  _el365.appendChild(_t366);
  _el363.appendChild(_el365);
  const _t367 = document.createTextNode("");
  const _update__t367 = () => { let _res = count_parity(); if (typeof _res === 'function') _res = _res(); _t367.textContent = String(_res); };
  _subscribe("count", _update__t367);
  _update__t367();
  _el363.appendChild(_t367);
  _el357.appendChild(_el363);
  _el342.appendChild(_el357);
  _el341.appendChild(_el342);
  return AppLayout(_el341);
}

function _render_page_about() {
  document.title = "Flame Web Showcase";
  const _el368 = document.createElement("div");
  _el368.className = "page-container";
  const _el369 = document.createElement("main");
  _el369.className = "content-section";
  const _el370 = document.createElement("h1");
  _el370.className = "section-title";
  const _t371 = document.createTextNode("About Flame Web");
  _el370.appendChild(_t371);
  _el369.appendChild(_el370);
  const _el372 = document.createElement("p");
  _el372.className = "section-description";
  const _t373 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el372.appendChild(_t373);
  _el369.appendChild(_el372);
  const _el374 = document.createElement("div");
  _el374.className = "features-grid";
  const _el375 = document.createElement("div");
  _el375.className = "feature-card";
  const _el376 = document.createElement("div");
  _el376.className = "feature-icon";
  const _t377 = document.createTextNode("⚡");
  _el376.appendChild(_t377);
  _el375.appendChild(_el376);
  const _el378 = document.createElement("h3");
  const _t379 = document.createTextNode("Fine-Grained Signals");
  _el378.appendChild(_t379);
  _el375.appendChild(_el378);
  const _el380 = document.createElement("p");
  const _t381 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el380.appendChild(_t381);
  _el375.appendChild(_el380);
  _el374.appendChild(_el375);
  const _el382 = document.createElement("div");
  _el382.className = "feature-card";
  const _el383 = document.createElement("div");
  _el383.className = "feature-icon";
  const _t384 = document.createTextNode("🌐");
  _el383.appendChild(_t384);
  _el382.appendChild(_el383);
  const _el385 = document.createElement("h3");
  const _t386 = document.createTextNode("Built-in SPA Router");
  _el385.appendChild(_t386);
  _el382.appendChild(_el385);
  const _el387 = document.createElement("p");
  const _t388 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el387.appendChild(_t388);
  _el382.appendChild(_el387);
  _el374.appendChild(_el382);
  const _el389 = document.createElement("div");
  _el389.className = "feature-card";
  const _el390 = document.createElement("div");
  _el390.className = "feature-icon";
  const _t391 = document.createTextNode("🎨");
  _el390.appendChild(_t391);
  _el389.appendChild(_el390);
  const _el392 = document.createElement("h3");
  const _t393 = document.createTextNode("Scoped & Dynamic Styling");
  _el392.appendChild(_t393);
  _el389.appendChild(_el392);
  const _el394 = document.createElement("p");
  const _t395 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el394.appendChild(_t395);
  _el389.appendChild(_el394);
  _el374.appendChild(_el389);
  _el369.appendChild(_el374);
  _el368.appendChild(_el369);
  return AppLayout(_el368);
}

function _render_page_compute() {
  document.title = "Flame Web Showcase";
  const _el396 = document.createElement("div");
  _el396.className = "page-container";
  const _el397 = document.createElement("main");
  _el397.className = "content-section";
  const _el398 = document.createElement("h1");
  _el398.className = "section-title";
  const _t399 = document.createTextNode("⚡ WebAssembly, Compute & Web Annotations");
  _el398.appendChild(_t399);
  _el397.appendChild(_el398);
  const _el400 = document.createElement("p");
  _el400.className = "section-description";
  const _t401 = document.createTextNode("Flame combines fine-grained reactive HTML with high-speed WebAssembly routines and pure mathematical evaluation.");
  _el400.appendChild(_t401);
  _el397.appendChild(_el400);
  const _el402 = document.createElement("div");
  _el402.className = "compute-grid";
  const _el403 = document.createElement("div");
  _el403.className = "compute-card";
  const _el404 = document.createElement("span");
  _el404.className = "badge-wasm";
  const _t405 = document.createTextNode("@Wasm · WebAssembly");
  _el404.appendChild(_t405);
  _el403.appendChild(_el404);
  const _el406 = document.createElement("h2");
  _el406.className = "card-title";
  const _t407 = document.createTextNode("Native WASM VM Engine");
  _el406.appendChild(_t407);
  _el403.appendChild(_el406);
  const _el408 = document.createElement("p");
  _el408.className = "card-desc";
  const _t409 = document.createTextNode("Heavy mathematical routines are compiled directly to WebAssembly binary bytecode and instantiated in the browser.");
  _el408.appendChild(_t409);
  _el403.appendChild(_el408);
  const _el410 = document.createElement("div");
  _el410.className = "compute-val";
  const _t411 = document.createTextNode("");
  const _update__t411 = () => { let _res = _getSignal("wasm_output"); if (typeof _res === 'function') _res = _res(); _t411.textContent = String(_res); };
  _subscribe("wasm_output", _update__t411);
  _update__t411();
  _el410.appendChild(_t411);
  _el403.appendChild(_el410);
  const _el412 = document.createElement("div");
  _el412.className = "stats-row";
  const _el413 = document.createElement("span");
  _el413.className = "badge-wasm";
  const _t414 = document.createTextNode("");
  const _update__t414 = () => { let _res = _getSignal("wasm_speed_badge"); if (typeof _res === 'function') _res = _res(); _t414.textContent = String(_res); };
  _subscribe("wasm_speed_badge", _update__t414);
  _update__t414();
  _el413.appendChild(_t414);
  _el412.appendChild(_el413);
  _el403.appendChild(_el412);
  const _el415 = document.createElement("div");
  _el415.className = "compute-actions";
  const _el416 = document.createElement("button");
  _el416.addEventListener("click", (event) => { runWasmFib(event); });
  _el416.className = "btn-calc";
  const _t417 = document.createTextNode("Run Fib(");
  _el416.appendChild(_t417);
  const _t418 = document.createTextNode("");
  const _update__t418 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t418.textContent = String(_res); };
  _subscribe("compute_num", _update__t418);
  _update__t418();
  _el416.appendChild(_t418);
  const _t419 = document.createTextNode(")");
  _el416.appendChild(_t419);
  _el415.appendChild(_el416);
  const _el420 = document.createElement("button");
  _el420.addEventListener("click", (event) => { runWasmFactorial(event); });
  _el420.className = "btn-calc";
  const _t421 = document.createTextNode("Run Factorial(");
  _el420.appendChild(_t421);
  const _t422 = document.createTextNode("");
  const _update__t422 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t422.textContent = String(_res); };
  _subscribe("compute_num", _update__t422);
  _update__t422();
  _el420.appendChild(_t422);
  const _t423 = document.createTextNode(")");
  _el420.appendChild(_t423);
  _el415.appendChild(_el420);
  _el403.appendChild(_el415);
  _el402.appendChild(_el403);
  const _el424 = document.createElement("div");
  _el424.className = "compute-card";
  const _el425 = document.createElement("span");
  _el425.className = "badge-computed";
  const _t426 = document.createTextNode("@Computed · Reactive Signals");
  _el425.appendChild(_t426);
  _el424.appendChild(_el425);
  const _el427 = document.createElement("h2");
  _el427.className = "card-title";
  const _t428 = document.createTextNode("Reactive Derived Computations");
  _el427.appendChild(_t428);
  _el424.appendChild(_el427);
  const _el429 = document.createElement("p");
  _el429.className = "card-desc";
  const _t430 = document.createTextNode("Automatically tracks dependencies on @State variables and recalculates with zero virtual DOM overhead.");
  _el429.appendChild(_t430);
  _el424.appendChild(_el429);
  const _el431 = document.createElement("div");
  _el431.className = "compute-val";
  const _t432 = document.createTextNode("Active Value:");
  _el431.appendChild(_t432);
  const _t433 = document.createTextNode("");
  const _update__t433 = () => { let _res = _getSignal("compute_num"); if (typeof _res === 'function') _res = _res(); _t433.textContent = String(_res); };
  _subscribe("compute_num", _update__t433);
  _update__t433();
  _el431.appendChild(_t433);
  _el424.appendChild(_el431);
  const _el434 = document.createElement("div");
  _el434.className = "stats-row";
  const _el435 = document.createElement("div");
  _el435.className = "stat-pill";
  const _t436 = document.createTextNode("Double:");
  _el435.appendChild(_t436);
  const _t437 = document.createTextNode("");
  const _update__t437 = () => { let _res = double_compute_num(); if (typeof _res === 'function') _res = _res(); _t437.textContent = String(_res); };
  _subscribe("compute_num", _update__t437);
  _update__t437();
  _el435.appendChild(_t437);
  _el434.appendChild(_el435);
  const _el438 = document.createElement("div");
  _el438.className = "stat-pill";
  const _t439 = document.createTextNode("Squared:");
  _el438.appendChild(_t439);
  const _t440 = document.createTextNode("");
  const _update__t440 = () => { let _res = square_compute_num(); if (typeof _res === 'function') _res = _res(); _t440.textContent = String(_res); };
  _subscribe("compute_num", _update__t440);
  _update__t440();
  _el438.appendChild(_t440);
  _el434.appendChild(_el438);
  const _el441 = document.createElement("div");
  _el441.className = "stat-pill";
  const _t442 = document.createTextNode("Parity:");
  _el441.appendChild(_t442);
  const _t443 = document.createTextNode("");
  const _update__t443 = () => { let _res = compute_num_parity(); if (typeof _res === 'function') _res = _res(); _t443.textContent = String(_res); };
  _subscribe("compute_num", _update__t443);
  _update__t443();
  _el441.appendChild(_t443);
  _el434.appendChild(_el441);
  _el424.appendChild(_el434);
  const _el444 = document.createElement("div");
  _el444.className = "compute-actions";
  const _el445 = document.createElement("button");
  _el445.addEventListener("click", (event) => { incrementNum(event); });
  _el445.className = "btn-calc";
  const _t446 = document.createTextNode("Increment (+1)");
  _el445.appendChild(_t446);
  _el444.appendChild(_el445);
  const _el447 = document.createElement("button");
  _el447.addEventListener("click", (event) => { decrementNum(event); });
  _el447.className = "btn-calc";
  const _t448 = document.createTextNode("Decrement (-1)");
  _el447.appendChild(_t448);
  _el444.appendChild(_el447);
  _el424.appendChild(_el444);
  _el402.appendChild(_el424);
  const _el449 = document.createElement("div");
  _el449.className = "compute-card";
  const _el450 = document.createElement("span");
  _el450.className = "badge-compute";
  const _t451 = document.createTextNode("@Compute · Pure Algorithms");
  _el450.appendChild(_t451);
  _el449.appendChild(_el450);
  const _el452 = document.createElement("h2");
  _el452.className = "card-title";
  const _t453 = document.createTextNode("Optimized Math Routines");
  _el452.appendChild(_t453);
  _el449.appendChild(_el452);
  const _el454 = document.createElement("p");
  _el454.className = "card-desc";
  const _t455 = document.createTextNode("Optimized mathematical calculations executed with maximum efficiency.");
  _el454.appendChild(_t455);
  _el449.appendChild(_el454);
  const _el456 = document.createElement("div");
  _el456.className = "stats-row";
  const _el457 = document.createElement("div");
  _el457.className = "stat-pill";
  const _t458 = document.createTextNode("Prime Test:");
  _el457.appendChild(_t458);
  const _t459 = document.createTextNode("");
  const _update__t459 = () => { let _res = prime_status(); if (typeof _res === 'function') _res = _res(); _t459.textContent = String(_res); };
  _subscribe("compute_num", _update__t459);
  _update__t459();
  _el457.appendChild(_t459);
  _el456.appendChild(_el457);
  _el449.appendChild(_el456);
  const _el460 = document.createElement("div");
  _el460.className = "stats-row";
  const _el461 = document.createElement("div");
  _el461.className = "stat-pill";
  const _t462 = document.createTextNode("Collatz Sequence:");
  _el461.appendChild(_t462);
  const _t463 = document.createTextNode("");
  const _update__t463 = () => { let _res = compute_collatz_steps(_getSignal("compute_num")); if (typeof _res === 'function') _res = _res(); _t463.textContent = String(_res); };
  _subscribe("compute_num", _update__t463);
  _update__t463();
  _el461.appendChild(_t463);
  const _t464 = document.createTextNode("steps");
  _el461.appendChild(_t464);
  _el460.appendChild(_el461);
  _el449.appendChild(_el460);
  const _el465 = document.createElement("div");
  _el465.className = "compute-actions";
  const _el466 = document.createElement("button");
  _el466.addEventListener("click", (event) => { setPrimeTest(event); });
  _el466.className = "btn-calc";
  const _t467 = document.createTextNode("Test Prime (29)");
  _el466.appendChild(_t467);
  _el465.appendChild(_el466);
  const _el468 = document.createElement("button");
  _el468.addEventListener("click", (event) => { setCollatzTest(event); });
  _el468.className = "btn-calc";
  const _t469 = document.createTextNode("Test Collatz (27)");
  _el468.appendChild(_t469);
  _el465.appendChild(_el468);
  _el449.appendChild(_el465);
  _el402.appendChild(_el449);
  _el397.appendChild(_el402);
  _el396.appendChild(_el397);
  return AppLayout(_el396);
}

function _render_page_contact() {
  document.title = "Flame Web Showcase";
  const _el470 = document.createElement("div");
  _el470.className = "page-container";
  const _el471 = document.createElement("main");
  _el471.className = "content-section";
  const _el472 = document.createElement("h1");
  _el472.className = "section-title";
  const _t473 = document.createTextNode("Get in Touch");
  _el472.appendChild(_t473);
  _el471.appendChild(_el472);
  const _el474 = document.createElement("p");
  _el474.className = "section-description";
  const _t475 = document.createTextNode("Send a message through our reactive contact form.");
  _el474.appendChild(_t475);
  _el471.appendChild(_el474);
  const _el476 = document.createElement("div");
  _el476.className = "contact-box";
  const _el477 = document.createElement("div");
  _el477.className = "form-group";
  const _el478 = document.createElement("label");
  const _t479 = document.createTextNode("Your Name");
  _el478.appendChild(_t479);
  _el477.appendChild(_el478);
  const _el480 = document.createElement("input");
  _el480.setAttribute("id", "contact-name");
  _el480.setAttribute("type", "text");
  _el480.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el480.className = "input-field";
  _el477.appendChild(_el480);
  _el476.appendChild(_el477);
  const _el481 = document.createElement("div");
  _el481.className = "form-group";
  const _el482 = document.createElement("label");
  const _t483 = document.createTextNode("Message");
  _el482.appendChild(_t483);
  _el481.appendChild(_el482);
  const _el484 = document.createElement("textarea");
  _el484.setAttribute("id", "contact-msg");
  _el484.setAttribute("placeholder", "Tell us about your project...");
  _el484.className = "input-field textarea";
  _el481.appendChild(_el484);
  _el476.appendChild(_el481);
  const _el485 = document.createElement("button");
  _el485.addEventListener("click", (event) => { submitContact(event); });
  _el485.className = "btn-primary";
  const _t486 = document.createTextNode("Send Message");
  _el485.appendChild(_t486);
  _el476.appendChild(_el485);
  const _el487 = document.createElement("div");
  _el487.className = "status-banner";
  const _t488 = document.createTextNode("Status:");
  _el487.appendChild(_t488);
  const _t489 = document.createTextNode("");
  const _update__t489 = () => { let _res = _getSignal("contact_status"); if (typeof _res === 'function') _res = _res(); _t489.textContent = String(_res); };
  _subscribe("contact_status", _update__t489);
  _update__t489();
  _el487.appendChild(_t489);
  _el476.appendChild(_el487);
  _el471.appendChild(_el476);
  _el470.appendChild(_el471);
  return AppLayout(_el470);
}

function _render_page_projects() {
  document.title = "Flame Web Showcase";
  const _el490 = document.createElement("div");
  _el490.className = "page-container";
  const _el491 = document.createElement("main");
  _el491.className = "content-section";
  const _el492 = document.createElement("h1");
  _el492.className = "section-title";
  const _t493 = document.createTextNode("Live API Projects");
  _el492.appendChild(_t493);
  _el491.appendChild(_el492);
  const _el494 = document.createElement("p");
  _el494.className = "section-description";
  const _t495 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el494.appendChild(_t495);
  _el491.appendChild(_el494);
  const _el496 = document.createElement("div");
  _el496.className = "action-bar";
  const _el497 = document.createElement("button");
  _el497.addEventListener("click", (event) => { fetchProjects(event); });
  _el497.className = "btn-primary";
  const _t498 = document.createTextNode("Fetch Live Posts via http.get()");
  _el497.appendChild(_t498);
  _el496.appendChild(_el497);
  _el491.appendChild(_el496);
  const _el499 = document.createElement("div");
  _el499.setAttribute("id", "projects-grid");
  _el499.className = "projects-container";
  const _el500 = document.createElement("div");
  _el500.className = "empty-state";
  const _t501 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el500.appendChild(_t501);
  _el499.appendChild(_el500);
  _el491.appendChild(_el499);
  _el490.appendChild(_el491);
  return AppLayout(_el490);
}

// Client-Side Router
const _routes = [
  { path: "/", render: _render_page_index },
  { path: "/about", render: _render_page_about },
  { path: "/compute", render: _render_page_compute },
  { path: "/contact", render: _render_page_contact },
  { path: "/projects", render: _render_page_projects },
];

function _renderActiveRoute() {
  const currentPath = window.location.pathname || "/";
  const appRoot = document.getElementById("app");
  if (!appRoot) return;

  let match = _routes.find(r => r.path === currentPath);
  if (!match) {
    match = _routes.find(r => r.path === "/");
  }
  if (!match && _routes.length > 0) {
    match = _routes[0];
  }

  appRoot.innerHTML = "";
  if (match) {
    const el = match.render();
    if (el) {
      appRoot.appendChild(el);
    }
  }
}

// Initial mount on DOM load
if (document.readyState === "loading") {
  document.addEventListener("DOMContentLoaded", _renderActiveRoute);
} else {
  _renderActiveRoute();
}
