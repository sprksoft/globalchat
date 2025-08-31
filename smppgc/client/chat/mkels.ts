// @ts-nocheck
const STICKERS = ["404", "spinny"];
const IMAGE_STICKERS = ["nightmarebirb", "smpp", "smppoud", "smpplite", "gc", "fire", "arch", "tux", "ferris", "gopher", "keith", "slonik", "mobydock",]; // avail stickers (used to prevent unneeded 404s to the server)


export function mksender(sender, parent_el) {
  let special = sender == "system";
  let sender_el = document.createElement("span");
  if (special) {
    sender_el.classList.add("special");
  }
  sender_el.classList.add("user");
  sender_el.innerText = sender;
  parent_el.appendChild(sender_el);
}
export function mkspace(parent_el) {
  let space = document.createElement("div");
  space.classList.add("space");
  parent_el.appendChild(space);
}

export function mktime(time, parent_el) {
  if (time == undefined) { return; }
  let time_el = document.createElement("small");
  time_el.classList.add("message_timestamp")
  time_el.innerText = time.toLocaleString(undefined, {
    dateStyle: "short",
    timeStyle: "short",
  });
  parent_el.appendChild(time_el);
}
export function mkspan(innerText) {
  let span = document.createElement("span");
  span.innerText = innerText;
  return span;
}
export function mka(link, parent_el) {
  let a = document.createElement("a");
  a.href = link;
  a.target = "_blank";
  a.innerText = link;
  parent_el.appendChild(a);
}

export function mkimgsticker(name: string): HTMLElement {
  let img = document.createElement("img");
  if (name == "keith") {
    img.height = 55;
  } else {
    img.height = 50;
  }
  img.src = "/static/stickies/" + name + ".webp";
  return img;
}

export function mksticker(name: string): HTMLElement {
  let el;
  if (IMAGE_STICKERS.includes(name)) {
    el = mkimgsticker(name);
  } else if (STICKERS.includes(name)) {
    switch (name) {
      case "404":
        let link = document.createElement("a");
        link.appendChild(mkimgsticker("404"));
        link.target = "_blank";
        link.href = "https://www.youtube.com/watch?v=dQw4w9WgXcQ";
        el = link;
        break;
      case "spinny":
        el = mkspan("🚁");
        break;
    }
  }
  if (el) {
    el.classList.add("sticker-" + name);
    el.dataset.sticker = name;
  }
  return el;
}
