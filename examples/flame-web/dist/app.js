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

_createSignal("btn_color", "#3b82f6");
_createSignal("count", 0);
_createSignal("contact_status", "Awaiting your message");

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
setupCursor();

function submitContact() {
  _setSignal("contact_status", "🚀 Thank you! Your message was sent successfully.");
}

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

function Projects() {
  const _el0 = document.createElement("div");
  _el0.className = "page-container";
  const _el1 = document.createElement("main");
  _el1.className = "content-section";
  const _el2 = document.createElement("h1");
  _el2.className = "section-title";
  const _t3 = document.createTextNode("Live API Projects");
  _el2.appendChild(_t3);
  _el1.appendChild(_el2);
  const _el4 = document.createElement("p");
  _el4.className = "section-description";
  const _t5 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el4.appendChild(_t5);
  _el1.appendChild(_el4);
  const _el6 = document.createElement("div");
  _el6.className = "action-bar";
  const _el7 = document.createElement("button");
  _el7.addEventListener("click", (event) => { fetchProjects(event); });
  _el7.className = "btn-primary";
  const _t8 = document.createTextNode("Fetch Live Posts via http.get()");
  _el7.appendChild(_t8);
  _el6.appendChild(_el7);
  _el1.appendChild(_el6);
  const _el9 = document.createElement("div");
  _el9.setAttribute("id", "projects-grid");
  _el9.className = "projects-container";
  const _el10 = document.createElement("div");
  _el10.className = "empty-state";
  const _t11 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el10.appendChild(_t11);
  _el9.appendChild(_el10);
  _el1.appendChild(_el9);
  _el0.appendChild(_el1);
  return _el0;
}

function index() {
  const _el12 = document.createElement("div");
  _el12.className = "page-container";
  const _el13 = document.createElement("main");
  _el13.className = "hero-section";
  const _el14 = document.createElement("div");
  _el14.className = "hero-badge";
  const _t15 = document.createTextNode("Next-Gen Reactive Framework");
  _el14.appendChild(_t15);
  _el13.appendChild(_el14);
  const _el16 = document.createElement("h1");
  _el16.className = "hero-title";
  const _t17 = document.createTextNode("High-Performance Web in Flame");
  _el16.appendChild(_t17);
  _el13.appendChild(_el16);
  const _el18 = document.createElement("p");
  _el18.className = "hero-subtitle";
  const _t19 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity and zero-overhead routing.");
  _el18.appendChild(_t19);
  _el13.appendChild(_el18);
  const _el20 = document.createElement("div");
  _el20.className = "interactive-panel";
  const _el21 = document.createElement("button");
  _el21.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el21.className = "btn-counter";
  const _t22 = document.createTextNode("Counter:");
  _el21.appendChild(_t22);
  const _t23 = document.createTextNode("");
  const _update__t23 = () => { _t23.textContent = String(_getSignal("count")); };
  _subscribe("count", _update__t23);
  _update__t23();
  _el21.appendChild(_t23);
  _el20.appendChild(_el21);
  const _el24 = document.createElement("button");
  _el24.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_25 = () => { _el24.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_25);
  _update_attr_25();
  _el24.className = "btn-color-toggle";
  const _t26 = document.createTextNode("Active Color:");
  _el24.appendChild(_t26);
  const _t27 = document.createTextNode("");
  const _update__t27 = () => { _t27.textContent = String(_getSignal("btn_color")); };
  _subscribe("btn_color", _update__t27);
  _update__t27();
  _el24.appendChild(_t27);
  _el20.appendChild(_el24);
  _el13.appendChild(_el20);
  _el12.appendChild(_el13);
  return _el12;
}

function projects() {
  const _el28 = document.createElement("div");
  _el28.className = "page-container";
  const _el29 = document.createElement("main");
  _el29.className = "content-section";
  const _el30 = document.createElement("h1");
  _el30.className = "section-title";
  const _t31 = document.createTextNode("Live API Projects");
  _el30.appendChild(_t31);
  _el29.appendChild(_el30);
  const _el32 = document.createElement("p");
  _el32.className = "section-description";
  const _t33 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el32.appendChild(_t33);
  _el29.appendChild(_el32);
  const _el34 = document.createElement("div");
  _el34.className = "action-bar";
  const _el35 = document.createElement("button");
  _el35.addEventListener("click", (event) => { fetchProjects(event); });
  _el35.className = "btn-primary";
  const _t36 = document.createTextNode("Fetch Live Posts via http.get()");
  _el35.appendChild(_t36);
  _el34.appendChild(_el35);
  _el29.appendChild(_el34);
  const _el37 = document.createElement("div");
  _el37.setAttribute("id", "projects-grid");
  _el37.className = "projects-container";
  const _el38 = document.createElement("div");
  _el38.className = "empty-state";
  const _t39 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el38.appendChild(_t39);
  _el37.appendChild(_el38);
  _el29.appendChild(_el37);
  _el28.appendChild(_el29);
  return _el28;
}

function about() {
  const _el40 = document.createElement("div");
  _el40.className = "page-container";
  const _el41 = document.createElement("main");
  _el41.className = "content-section";
  const _el42 = document.createElement("h1");
  _el42.className = "section-title";
  const _t43 = document.createTextNode("About Flame Web");
  _el42.appendChild(_t43);
  _el41.appendChild(_el42);
  const _el44 = document.createElement("p");
  _el44.className = "section-description";
  const _t45 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el44.appendChild(_t45);
  _el41.appendChild(_el44);
  const _el46 = document.createElement("div");
  _el46.className = "features-grid";
  const _el47 = document.createElement("div");
  _el47.className = "feature-card";
  const _el48 = document.createElement("div");
  _el48.className = "feature-icon";
  const _t49 = document.createTextNode("⚡");
  _el48.appendChild(_t49);
  _el47.appendChild(_el48);
  const _el50 = document.createElement("h3");
  const _t51 = document.createTextNode("Fine-Grained Signals");
  _el50.appendChild(_t51);
  _el47.appendChild(_el50);
  const _el52 = document.createElement("p");
  const _t53 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el52.appendChild(_t53);
  _el47.appendChild(_el52);
  _el46.appendChild(_el47);
  const _el54 = document.createElement("div");
  _el54.className = "feature-card";
  const _el55 = document.createElement("div");
  _el55.className = "feature-icon";
  const _t56 = document.createTextNode("🌐");
  _el55.appendChild(_t56);
  _el54.appendChild(_el55);
  const _el57 = document.createElement("h3");
  const _t58 = document.createTextNode("Built-in SPA Router");
  _el57.appendChild(_t58);
  _el54.appendChild(_el57);
  const _el59 = document.createElement("p");
  const _t60 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el59.appendChild(_t60);
  _el54.appendChild(_el59);
  _el46.appendChild(_el54);
  const _el61 = document.createElement("div");
  _el61.className = "feature-card";
  const _el62 = document.createElement("div");
  _el62.className = "feature-icon";
  const _t63 = document.createTextNode("🎨");
  _el62.appendChild(_t63);
  _el61.appendChild(_el62);
  const _el64 = document.createElement("h3");
  const _t65 = document.createTextNode("Scoped & Dynamic Styling");
  _el64.appendChild(_t65);
  _el61.appendChild(_el64);
  const _el66 = document.createElement("p");
  const _t67 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el66.appendChild(_t67);
  _el61.appendChild(_el66);
  _el46.appendChild(_el61);
  _el41.appendChild(_el46);
  _el40.appendChild(_el41);
  return _el40;
}

function contact() {
  const _el68 = document.createElement("div");
  _el68.className = "page-container";
  const _el69 = document.createElement("main");
  _el69.className = "content-section";
  const _el70 = document.createElement("h1");
  _el70.className = "section-title";
  const _t71 = document.createTextNode("Get in Touch");
  _el70.appendChild(_t71);
  _el69.appendChild(_el70);
  const _el72 = document.createElement("p");
  _el72.className = "section-description";
  const _t73 = document.createTextNode("Send a message through our reactive contact form.");
  _el72.appendChild(_t73);
  _el69.appendChild(_el72);
  const _el74 = document.createElement("div");
  _el74.className = "contact-box";
  const _el75 = document.createElement("div");
  _el75.className = "form-group";
  const _el76 = document.createElement("label");
  const _t77 = document.createTextNode("Your Name");
  _el76.appendChild(_t77);
  _el75.appendChild(_el76);
  const _el78 = document.createElement("input");
  _el78.setAttribute("id", "contact-name");
  _el78.setAttribute("type", "text");
  _el78.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el78.className = "input-field";
  _el75.appendChild(_el78);
  _el74.appendChild(_el75);
  const _el79 = document.createElement("div");
  _el79.className = "form-group";
  const _el80 = document.createElement("label");
  const _t81 = document.createTextNode("Message");
  _el80.appendChild(_t81);
  _el79.appendChild(_el80);
  const _el82 = document.createElement("textarea");
  _el82.setAttribute("id", "contact-msg");
  _el82.setAttribute("placeholder", "Tell us about your project...");
  _el82.className = "input-field textarea";
  _el79.appendChild(_el82);
  _el74.appendChild(_el79);
  const _el83 = document.createElement("button");
  _el83.addEventListener("click", (event) => { submitContact(event); });
  _el83.className = "btn-primary";
  const _t84 = document.createTextNode("Send Message");
  _el83.appendChild(_t84);
  _el74.appendChild(_el83);
  const _el85 = document.createElement("div");
  _el85.className = "status-banner";
  const _t86 = document.createTextNode("Status:");
  _el85.appendChild(_t86);
  const _t87 = document.createTextNode("");
  const _update__t87 = () => { _t87.textContent = String(_getSignal("contact_status")); };
  _subscribe("contact_status", _update__t87);
  _update__t87();
  _el85.appendChild(_t87);
  _el74.appendChild(_el85);
  _el69.appendChild(_el74);
  _el68.appendChild(_el69);
  return _el68;
}

function Contact() {
  const _el88 = document.createElement("div");
  _el88.className = "page-container";
  const _el89 = document.createElement("main");
  _el89.className = "content-section";
  const _el90 = document.createElement("h1");
  _el90.className = "section-title";
  const _t91 = document.createTextNode("Get in Touch");
  _el90.appendChild(_t91);
  _el89.appendChild(_el90);
  const _el92 = document.createElement("p");
  _el92.className = "section-description";
  const _t93 = document.createTextNode("Send a message through our reactive contact form.");
  _el92.appendChild(_t93);
  _el89.appendChild(_el92);
  const _el94 = document.createElement("div");
  _el94.className = "contact-box";
  const _el95 = document.createElement("div");
  _el95.className = "form-group";
  const _el96 = document.createElement("label");
  const _t97 = document.createTextNode("Your Name");
  _el96.appendChild(_t97);
  _el95.appendChild(_el96);
  const _el98 = document.createElement("input");
  _el98.setAttribute("id", "contact-name");
  _el98.setAttribute("type", "text");
  _el98.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el98.className = "input-field";
  _el95.appendChild(_el98);
  _el94.appendChild(_el95);
  const _el99 = document.createElement("div");
  _el99.className = "form-group";
  const _el100 = document.createElement("label");
  const _t101 = document.createTextNode("Message");
  _el100.appendChild(_t101);
  _el99.appendChild(_el100);
  const _el102 = document.createElement("textarea");
  _el102.setAttribute("id", "contact-msg");
  _el102.setAttribute("placeholder", "Tell us about your project...");
  _el102.className = "input-field textarea";
  _el99.appendChild(_el102);
  _el94.appendChild(_el99);
  const _el103 = document.createElement("button");
  _el103.addEventListener("click", (event) => { submitContact(event); });
  _el103.className = "btn-primary";
  const _t104 = document.createTextNode("Send Message");
  _el103.appendChild(_t104);
  _el94.appendChild(_el103);
  const _el105 = document.createElement("div");
  _el105.className = "status-banner";
  const _t106 = document.createTextNode("Status:");
  _el105.appendChild(_t106);
  const _t107 = document.createTextNode("");
  const _update__t107 = () => { _t107.textContent = String(_getSignal("contact_status")); };
  _subscribe("contact_status", _update__t107);
  _update__t107();
  _el105.appendChild(_t107);
  _el94.appendChild(_el105);
  _el89.appendChild(_el94);
  _el88.appendChild(_el89);
  return _el88;
}

function Index() {
  const _el108 = document.createElement("div");
  _el108.className = "page-container";
  const _el109 = document.createElement("main");
  _el109.className = "hero-section";
  const _el110 = document.createElement("div");
  _el110.className = "hero-badge";
  const _t111 = document.createTextNode("Next-Gen Reactive Framework");
  _el110.appendChild(_t111);
  _el109.appendChild(_el110);
  const _el112 = document.createElement("h1");
  _el112.className = "hero-title";
  const _t113 = document.createTextNode("High-Performance Web in Flame");
  _el112.appendChild(_t113);
  _el109.appendChild(_el112);
  const _el114 = document.createElement("p");
  _el114.className = "hero-subtitle";
  const _t115 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity and zero-overhead routing.");
  _el114.appendChild(_t115);
  _el109.appendChild(_el114);
  const _el116 = document.createElement("div");
  _el116.className = "interactive-panel";
  const _el117 = document.createElement("button");
  _el117.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el117.className = "btn-counter";
  const _t118 = document.createTextNode("Counter:");
  _el117.appendChild(_t118);
  const _t119 = document.createTextNode("");
  const _update__t119 = () => { _t119.textContent = String(_getSignal("count")); };
  _subscribe("count", _update__t119);
  _update__t119();
  _el117.appendChild(_t119);
  _el116.appendChild(_el117);
  const _el120 = document.createElement("button");
  _el120.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_121 = () => { _el120.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_121);
  _update_attr_121();
  _el120.className = "btn-color-toggle";
  const _t122 = document.createTextNode("Active Color:");
  _el120.appendChild(_t122);
  const _t123 = document.createTextNode("");
  const _update__t123 = () => { _t123.textContent = String(_getSignal("btn_color")); };
  _subscribe("btn_color", _update__t123);
  _update__t123();
  _el120.appendChild(_t123);
  _el116.appendChild(_el120);
  _el109.appendChild(_el116);
  _el108.appendChild(_el109);
  return _el108;
}

function AppLayout(children) {
  const _el124 = document.createElement("div");
  _el124.className = "app-layout";
  const _comp125 = NavBar({});
  _el124.appendChild(_comp125);
  const _el126 = document.createElement("div");
  _el126.className = "app-content";
  const _val128 = children;
  if (typeof _val128 === 'object' && _val128 instanceof Node) {
    _el126.appendChild(_val128);
  } else if (Array.isArray(_val128)) {
    for (const _item of _val128) {
      if (typeof _item === 'object' && _item instanceof Node) {
        _el126.appendChild(_item);
      } else {
        _el126.appendChild(document.createTextNode(String(_item)));
      }
    }
  } else {
    _el126.appendChild(document.createTextNode(String(_val128)));
  }
  _el124.appendChild(_el126);
  return _el124;
}

function NavBar() {
  const _el129 = document.createElement("header");
  _el129.className = "navbar";
  const _el130 = document.createElement("div");
  _el130.setAttribute("id", "cursor-dot");
  _el130.className = "cursor-dot";
  _el129.appendChild(_el130);
  const _el131 = document.createElement("div");
  _el131.setAttribute("id", "cursor-outline");
  _el131.className = "cursor-outline";
  _el129.appendChild(_el131);
  const _el132 = document.createElement("div");
  _el132.className = "nav-brand";
  const _el133 = document.createElement("span");
  _el133.className = "brand-icon";
  const _t134 = document.createTextNode("⚡");
  _el133.appendChild(_t134);
  _el132.appendChild(_el133);
  const _el135 = document.createElement("span");
  _el135.className = "brand-text";
  const _t136 = document.createTextNode("Flame Web");
  _el135.appendChild(_t136);
  _el132.appendChild(_el135);
  _el129.appendChild(_el132);
  const _el137 = document.createElement("nav");
  _el137.className = "nav-links";
  const _el138 = document.createElement("a");
  _el138.setAttribute("href", "/");
  _el138.className = "nav-link";
  const _t139 = document.createTextNode("Home");
  _el138.appendChild(_t139);
  _el137.appendChild(_el138);
  const _el140 = document.createElement("a");
  _el140.setAttribute("href", "/about");
  _el140.className = "nav-link";
  const _t141 = document.createTextNode("About");
  _el140.appendChild(_t141);
  _el137.appendChild(_el140);
  const _el142 = document.createElement("a");
  _el142.setAttribute("href", "/projects");
  _el142.className = "nav-link";
  const _t143 = document.createTextNode("Projects");
  _el142.appendChild(_t143);
  _el137.appendChild(_el142);
  const _el144 = document.createElement("a");
  _el144.setAttribute("href", "/contact");
  _el144.className = "nav-link";
  const _t145 = document.createTextNode("Contact");
  _el144.appendChild(_t145);
  _el137.appendChild(_el144);
  _el129.appendChild(_el137);
  return _el129;
}

function About() {
  const _el146 = document.createElement("div");
  _el146.className = "page-container";
  const _el147 = document.createElement("main");
  _el147.className = "content-section";
  const _el148 = document.createElement("h1");
  _el148.className = "section-title";
  const _t149 = document.createTextNode("About Flame Web");
  _el148.appendChild(_t149);
  _el147.appendChild(_el148);
  const _el150 = document.createElement("p");
  _el150.className = "section-description";
  const _t151 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el150.appendChild(_t151);
  _el147.appendChild(_el150);
  const _el152 = document.createElement("div");
  _el152.className = "features-grid";
  const _el153 = document.createElement("div");
  _el153.className = "feature-card";
  const _el154 = document.createElement("div");
  _el154.className = "feature-icon";
  const _t155 = document.createTextNode("⚡");
  _el154.appendChild(_t155);
  _el153.appendChild(_el154);
  const _el156 = document.createElement("h3");
  const _t157 = document.createTextNode("Fine-Grained Signals");
  _el156.appendChild(_t157);
  _el153.appendChild(_el156);
  const _el158 = document.createElement("p");
  const _t159 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el158.appendChild(_t159);
  _el153.appendChild(_el158);
  _el152.appendChild(_el153);
  const _el160 = document.createElement("div");
  _el160.className = "feature-card";
  const _el161 = document.createElement("div");
  _el161.className = "feature-icon";
  const _t162 = document.createTextNode("🌐");
  _el161.appendChild(_t162);
  _el160.appendChild(_el161);
  const _el163 = document.createElement("h3");
  const _t164 = document.createTextNode("Built-in SPA Router");
  _el163.appendChild(_t164);
  _el160.appendChild(_el163);
  const _el165 = document.createElement("p");
  const _t166 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el165.appendChild(_t166);
  _el160.appendChild(_el165);
  _el152.appendChild(_el160);
  const _el167 = document.createElement("div");
  _el167.className = "feature-card";
  const _el168 = document.createElement("div");
  _el168.className = "feature-icon";
  const _t169 = document.createTextNode("🎨");
  _el168.appendChild(_t169);
  _el167.appendChild(_el168);
  const _el170 = document.createElement("h3");
  const _t171 = document.createTextNode("Scoped & Dynamic Styling");
  _el170.appendChild(_t171);
  _el167.appendChild(_el170);
  const _el172 = document.createElement("p");
  const _t173 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el172.appendChild(_t173);
  _el167.appendChild(_el172);
  _el152.appendChild(_el167);
  _el147.appendChild(_el152);
  _el146.appendChild(_el147);
  return _el146;
}

function _render_page_index() {
  document.title = "Flame Web Showcase";
  const _el174 = document.createElement("div");
  _el174.className = "page-container";
  const _el175 = document.createElement("main");
  _el175.className = "hero-section";
  const _el176 = document.createElement("div");
  _el176.className = "hero-badge";
  const _t177 = document.createTextNode("Next-Gen Reactive Framework");
  _el176.appendChild(_t177);
  _el175.appendChild(_el176);
  const _el178 = document.createElement("h1");
  _el178.className = "hero-title";
  const _t179 = document.createTextNode("High-Performance Web in Flame");
  _el178.appendChild(_t179);
  _el175.appendChild(_el178);
  const _el180 = document.createElement("p");
  _el180.className = "hero-subtitle";
  const _t181 = document.createTextNode("Compile Flame directly into ultra-fast JavaScript DOM code with fine-grained reactivity and zero-overhead routing.");
  _el180.appendChild(_t181);
  _el175.appendChild(_el180);
  const _el182 = document.createElement("div");
  _el182.className = "interactive-panel";
  const _el183 = document.createElement("button");
  _el183.addEventListener("click", (event) => { (_setSignal("count", s => s + (1))); });
  _el183.className = "btn-counter";
  const _t184 = document.createTextNode("Counter:");
  _el183.appendChild(_t184);
  const _t185 = document.createTextNode("");
  const _update__t185 = () => { _t185.textContent = String(_getSignal("count")); };
  _subscribe("count", _update__t185);
  _update__t185();
  _el183.appendChild(_t185);
  _el182.appendChild(_el183);
  const _el186 = document.createElement("button");
  _el186.addEventListener("click", (event) => { toggleColor(event); });
  const _update_attr_187 = () => { _el186.style.cssText = (("background-color: " + _getSignal("btn_color")) + ";"); };
  _subscribe("btn_color", _update_attr_187);
  _update_attr_187();
  _el186.className = "btn-color-toggle";
  const _t188 = document.createTextNode("Active Color:");
  _el186.appendChild(_t188);
  const _t189 = document.createTextNode("");
  const _update__t189 = () => { _t189.textContent = String(_getSignal("btn_color")); };
  _subscribe("btn_color", _update__t189);
  _update__t189();
  _el186.appendChild(_t189);
  _el182.appendChild(_el186);
  _el175.appendChild(_el182);
  _el174.appendChild(_el175);
  return AppLayout(_el174);
}

function _render_page_about() {
  document.title = "Flame Web Showcase";
  const _el190 = document.createElement("div");
  _el190.className = "page-container";
  const _el191 = document.createElement("main");
  _el191.className = "content-section";
  const _el192 = document.createElement("h1");
  _el192.className = "section-title";
  const _t193 = document.createTextNode("About Flame Web");
  _el192.appendChild(_t193);
  _el191.appendChild(_el192);
  const _el194 = document.createElement("p");
  _el194.className = "section-description";
  const _t195 = document.createTextNode("Flame Web pairs Flame syntax with a reactive DOM runtime. It compiles UI trees into native DOM calls with granular signals.");
  _el194.appendChild(_t195);
  _el191.appendChild(_el194);
  const _el196 = document.createElement("div");
  _el196.className = "features-grid";
  const _el197 = document.createElement("div");
  _el197.className = "feature-card";
  const _el198 = document.createElement("div");
  _el198.className = "feature-icon";
  const _t199 = document.createTextNode("⚡");
  _el198.appendChild(_t199);
  _el197.appendChild(_el198);
  const _el200 = document.createElement("h3");
  const _t201 = document.createTextNode("Fine-Grained Signals");
  _el200.appendChild(_t201);
  _el197.appendChild(_el200);
  const _el202 = document.createElement("p");
  const _t203 = document.createTextNode("No virtual DOM overhead. When state changes, only the exact DOM node updates.");
  _el202.appendChild(_t203);
  _el197.appendChild(_el202);
  _el196.appendChild(_el197);
  const _el204 = document.createElement("div");
  _el204.className = "feature-card";
  const _el205 = document.createElement("div");
  _el205.className = "feature-icon";
  const _t206 = document.createTextNode("🌐");
  _el205.appendChild(_t206);
  _el204.appendChild(_el205);
  const _el207 = document.createElement("h3");
  const _t208 = document.createTextNode("Built-in SPA Router");
  _el207.appendChild(_t208);
  _el204.appendChild(_el207);
  const _el209 = document.createElement("p");
  const _t210 = document.createTextNode("Multi-page navigation with client-side history state and instant page switches.");
  _el209.appendChild(_t210);
  _el204.appendChild(_el209);
  _el196.appendChild(_el204);
  const _el211 = document.createElement("div");
  _el211.className = "feature-card";
  const _el212 = document.createElement("div");
  _el212.className = "feature-icon";
  const _t213 = document.createTextNode("🎨");
  _el212.appendChild(_t213);
  _el211.appendChild(_el212);
  const _el214 = document.createElement("h3");
  const _t215 = document.createTextNode("Scoped & Dynamic Styling");
  _el214.appendChild(_t215);
  _el211.appendChild(_el214);
  const _el216 = document.createElement("p");
  const _t217 = document.createTextNode("Inline style attributes and @Style functions provide complete styling power.");
  _el216.appendChild(_t217);
  _el211.appendChild(_el216);
  _el196.appendChild(_el211);
  _el191.appendChild(_el196);
  _el190.appendChild(_el191);
  return AppLayout(_el190);
}

function _render_page_contact() {
  document.title = "Flame Web Showcase";
  const _el218 = document.createElement("div");
  _el218.className = "page-container";
  const _el219 = document.createElement("main");
  _el219.className = "content-section";
  const _el220 = document.createElement("h1");
  _el220.className = "section-title";
  const _t221 = document.createTextNode("Get in Touch");
  _el220.appendChild(_t221);
  _el219.appendChild(_el220);
  const _el222 = document.createElement("p");
  _el222.className = "section-description";
  const _t223 = document.createTextNode("Send a message through our reactive contact form.");
  _el222.appendChild(_t223);
  _el219.appendChild(_el222);
  const _el224 = document.createElement("div");
  _el224.className = "contact-box";
  const _el225 = document.createElement("div");
  _el225.className = "form-group";
  const _el226 = document.createElement("label");
  const _t227 = document.createTextNode("Your Name");
  _el226.appendChild(_t227);
  _el225.appendChild(_el226);
  const _el228 = document.createElement("input");
  _el228.setAttribute("id", "contact-name");
  _el228.setAttribute("type", "text");
  _el228.setAttribute("placeholder", "e.g. Satoshi Nakamoto");
  _el228.className = "input-field";
  _el225.appendChild(_el228);
  _el224.appendChild(_el225);
  const _el229 = document.createElement("div");
  _el229.className = "form-group";
  const _el230 = document.createElement("label");
  const _t231 = document.createTextNode("Message");
  _el230.appendChild(_t231);
  _el229.appendChild(_el230);
  const _el232 = document.createElement("textarea");
  _el232.setAttribute("id", "contact-msg");
  _el232.setAttribute("placeholder", "Tell us about your project...");
  _el232.className = "input-field textarea";
  _el229.appendChild(_el232);
  _el224.appendChild(_el229);
  const _el233 = document.createElement("button");
  _el233.addEventListener("click", (event) => { submitContact(event); });
  _el233.className = "btn-primary";
  const _t234 = document.createTextNode("Send Message");
  _el233.appendChild(_t234);
  _el224.appendChild(_el233);
  const _el235 = document.createElement("div");
  _el235.className = "status-banner";
  const _t236 = document.createTextNode("Status:");
  _el235.appendChild(_t236);
  const _t237 = document.createTextNode("");
  const _update__t237 = () => { _t237.textContent = String(_getSignal("contact_status")); };
  _subscribe("contact_status", _update__t237);
  _update__t237();
  _el235.appendChild(_t237);
  _el224.appendChild(_el235);
  _el219.appendChild(_el224);
  _el218.appendChild(_el219);
  return AppLayout(_el218);
}

function _render_page_projects() {
  document.title = "Flame Web Showcase";
  const _el238 = document.createElement("div");
  _el238.className = "page-container";
  const _el239 = document.createElement("main");
  _el239.className = "content-section";
  const _el240 = document.createElement("h1");
  _el240.className = "section-title";
  const _t241 = document.createTextNode("Live API Projects");
  _el240.appendChild(_t241);
  _el239.appendChild(_el240);
  const _el242 = document.createElement("p");
  _el242.className = "section-description";
  const _t243 = document.createTextNode("Demonstrating native std.net.http integration with jsonplaceholder.typicode.com/posts in Flame.");
  _el242.appendChild(_t243);
  _el239.appendChild(_el242);
  const _el244 = document.createElement("div");
  _el244.className = "action-bar";
  const _el245 = document.createElement("button");
  _el245.addEventListener("click", (event) => { fetchProjects(event); });
  _el245.className = "btn-primary";
  const _t246 = document.createTextNode("Fetch Live Posts via http.get()");
  _el245.appendChild(_t246);
  _el244.appendChild(_el245);
  _el239.appendChild(_el244);
  const _el247 = document.createElement("div");
  _el247.setAttribute("id", "projects-grid");
  _el247.className = "projects-container";
  const _el248 = document.createElement("div");
  _el248.className = "empty-state";
  const _t249 = document.createTextNode("Click the button above to load live posts using Flames std.net.http client.");
  _el248.appendChild(_t249);
  _el247.appendChild(_el248);
  _el239.appendChild(_el247);
  _el238.appendChild(_el239);
  return AppLayout(_el238);
}

// Client-Side Router
const _routes = [
  { path: "/", render: _render_page_index },
  { path: "/about", render: _render_page_about },
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
