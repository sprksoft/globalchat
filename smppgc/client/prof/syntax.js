import './css/syntax.css'

let str_to_token = {};
let token_to_str = {};
let escape_codes = [];


function errWrap(text, error) {
  return `<i class="special-token special-token-invalid" title="${error}">${text}</i>`;
}
function invalidTokenWrap(text) {
  let allowedStr = "a-z";
  for (let code of escape_codes) {
    allowedStr+=", /"+code;
  }
  return errWrap(text, "Invalid token (only "+allowedStr+" is allowed)");
}

function parse_tokengroup_syntax(str) {
  let synErr=false;
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
        synErr=true;
        parsed+=errWrap(`&#47;${char}`, "Invalid escape code");
      }
    } else {
      let cCode = char.charCodeAt(0);
      if (cCode >= 97 && cCode <= 122) {
        parsed+=char;
      }else {
        synErr=true;
        parsed+=invalidTokenWrap(char);
      }
    }

  }
  if (escape) {
    parsed+=invalidTokenWrap(`&#47;`);
  }
  return [parsed, synErr];
}

function parse(input) {
  let lang = input.parentElement.dataset.lang;
  let synErr=false;
  if (lang == "tokengroup") {
    let [parsed, hasErr] = parse_tokengroup_syntax(input.innerText);
    synErr = hasErr;
    input.previousElementSibling.innerHTML = parsed;
  } else if (lang == "none") {
    input.previousElementSibling.innerHTML = input.innerText.replace("\n", "");
  }else{
    return;
  }
  input.parentElement.dataset.syntaxerror=synErr;
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

export function setContent(editorEl, content) {
  let input = editorEl.querySelector(".editor_input")
  input.innerText=content;
  parse(input);
}
export function getContent(editorEl) {
  let input = editorEl.querySelector(".editor_input")
  return input.innerText;
}
export function focus(editorEl) {
  editorEl.querySelector(".editor_input").focus();
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

export function setTokenInfo(tokenInfo) {
  escape_codes = [];
  str_to_token = {};
  token_to_str = {};
  for (let token of tokenInfo) {
    str_to_token[token[0]] = [token[1], token[2]];
    token_to_str[token[1]] = ["/"+token[0], token[2]];
    escape_codes.push(token[0]);
  }
}


export function tokensToString(jsonTg) {
  let string = "";
  for (let token of jsonTg) {
    let entry = token_to_str[token];
    if (entry) {
      string+=entry[0];
    } else {
      string+=String.fromCharCode(token);
    }
  }
  return string;
}
export function stringToTokens(string) {
  let tokens = [];
  let escape = false;
  for (let char of string) {
    if (char == '/' && !escape) {
      escape=true;
      continue;
    }
    if (escape) {
      let result = str_to_token[char];
      if (!result) {
        return [];
      }
      tokens.push(result[0]);
      escape=false;
    } else {
      tokens.push(char.charCodeAt(0));
    }
  }
  if (escape) {
    tokens.push('/'.charCodeAt(0));
  }

  return tokens;
}
