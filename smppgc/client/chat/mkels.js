const STICKERS=["404", "arch", "tux", "smpp", "gc", "fire"]; // avail stickers (used to prevent unneeded 404s to the server)

export function mkProfHighlighted(message, start, end, parent_el) {
  let span = document.createElement("span");
  span.appendChild(document.createTextNode(message.substring(0, start)));
  let hi = document.createElement("mark");
  hi.classList.add("profanity-mark")
  hi.innerText = message.substring(start, end);
  span.appendChild(hi);
  span.appendChild(document.createTextNode(message.substring(end, message.length)));
  parent_el.appendChild(span);
}

export function mksender(sender, parent_el) {
  let special = sender == "system";
  let sender_el = document.createElement("span");
  if (special){
    sender_el.classList.add("special");
  }
  sender_el.classList.add("user");
  sender_el.innerText=sender;
  parent_el.appendChild(sender_el);
}
export function mkspace(parent_el) {
  let space = document.createElement("div");
  space.classList.add("space");
  parent_el.appendChild(space);
}

export function mktime(time, parent_el) {
  if (time == undefined){ return; }
  let time_el = document.createElement("small");
  time_el.classList.add("message_timestamp")
  time_el.innerText = time.toLocaleString(undefined, {
    dateStyle:"short",
    timeStyle:"short",
  });
  parent_el.appendChild(time_el);
}
export function mkspan(innerText, parent_el){
    let span = document.createElement("span");
    span.innerText=innerText;
    parent_el.appendChild(span);
}
export function mkprofmarkspan(innerText, parent_el) {
  let span = document.createElement("span");
  let mark = document.createElement("mark");
  mark.classList.add("profanity-mark");
  mark.innerText = innerText;
  span.appendChild(mark);
  parent_el.appendChild(span)
}
export function mka(link, parent_el) {
    let a = document.createElement("a");
    a.href=link;
    a.target="_blank";
    a.innerText=link;
    parent_el.appendChild(a);
}
export function mksticker(name, parent_el) {
    let img = document.createElement("img");
    img.width=50;
    img.dataset.sticker=name
    img.src=ROOT_URL+"/static/stickies/"+name+".webp";
    parent_el.appendChild(img);
}

// Parse the string message and generate html elements for stickers, links,...
export function mkcontent(message, highlight, parent_el) {
  if (highlight) {
    mkcontent(message.substring(0, highlight[0]), null, parent_el);
    mkprofmarkspan(message.substring(highlight[0], highlight[1]), parent_el)
    mkcontent(message.substring(highlight[1], message.length), null, parent_el);
  } else {
    const findStickerRegex = /:[a-z0-9_-]{1,10}:/g;
    const matches = message.matchAll(findStickerRegex);
    let last_index = 0;
    for (const match of matches) {
      let skip=false;
      mkspan(message.substring(last_index, match.index), parent_el);

      let name = match[0].substring(1, match[0].length-1);
      if (STICKERS.includes(name)) {
        mksticker(name, parent_el);
        last_index = match.index+match[0].length;
      }
    }
    mkspan(message.substring(last_index), parent_el);
  }
}
