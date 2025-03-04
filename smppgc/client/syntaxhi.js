import './syntaxhi.css'

let str_to_token = {};

function parse_tokengroup_syntax(str) {
  str = str.replace("\n", "");
  let parsed = "";
  let escape = false;
  for (let char of str) {
    if (char == '/' && !escape) {
      escape = true;
      continue;
    }
    if (escape) {
      escape = false;
      let entry = str_to_token[char];
      if (entry) {
        let desc = entry[1];
        parsed+=`<i class="special-token" title="${desc}">&#47;${char}</i>`
      } else {
        parsed+=`<i class="special-token special-token-invalid" title="Invalid token">&#47;${char}</i>`
      }
    } else {
      parsed+=char;
    }

  }
  if (escape) {
    parsed+="&#47;";
  }
  return parsed;
}

function parse(input) {
  let lang = input.parentElement.dataset.lang;
  if (lang == "tokengroup") {
    input.previousElementSibling.innerHTML = parse_tokengroup_syntax(input.innerText);
  } else if (lang == "none") {
    input.previousElementSibling.innerHTML = input.innerText.replace("\n", "");
  }else{
    return;
  }
  if (input.innerText.includes("\n")) {
    input.innerText = input.innerText.replace("\n", "");
  }
}

document.addEventListener("input", (e) => {
  parse(e.target);
});
document.addEventListener("click", (e) => {
  if (e.target.classList.contains("editor_highlight")) {
    e.target.nextElementSibling.click();
  }

});

export function setTokenInfo(tokenInfo) {
  for (let token of tokenInfo) {
    str_to_token[token[0]] = [token[1], token[2]];
  }
}

export function setContent(editorEl, content) {
  let input = editorEl.querySelector(".editor_input")
  input.innerText=content;
  parse(input);
}
export function getContent(editorEl) {
  let input = editorEl.querySelector(".editor_input")
  return input.innerText;
}

export function createEditor(parent) {
  if (parent.classList.contains("editor")){
    return;
  }
  parent.classList.add("editor");


  let highlightDiv = document.createElement("div");
  highlightDiv.classList.add("editor_highlight");
  let inputDiv = document.createElement("div");
  inputDiv.contentEditable = true;
  inputDiv.spellcheck = false;
  inputDiv.autocorrect = "off";
  inputDiv.autocapitalize = "off";
  inputDiv.dataset.plaintext=true;
  inputDiv.classList.add("editor_input");

  parent.appendChild(highlightDiv);
  parent.appendChild(inputDiv);
}

