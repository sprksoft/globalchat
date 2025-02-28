const STICKERS=["404", "arch", "tux", "smpp", "gc", "fire"]; // avail stickers (used to prevent unneeded 404s to the server)

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
export function mkcontent(message, parent_el) {
  const find_link_regex = /(https?:\/\/([-.a-z0-9]{1,60})(\/[-a-zA-Z0-9()@:%_\+.~#?&//=]{0,256})?)|(:[a-z0-9_-]{1,10}:)/g;
  const matches = message.matchAll(find_link_regex);
  let last_index = 0;
  for (const match of matches){
    let skip=false;
    mkspan(message.substring(last_index, match.index), parent_el);

    if (match[1] !== undefined){ // a link
      mka(match[1], parent_el)
    }
    if (match[4] !== undefined){ // a sticker
      let name = match[4].substring(1, match[4].length-1);
      if (STICKERS.includes(name)){
        mksticker(name, parent_el);
      }else{
        skip=true;
      }
    }
    if (!skip){
      last_index = match.index+match[0].length;
    }

  }
  mkspan(message.substring(last_index), parent_el);
}
