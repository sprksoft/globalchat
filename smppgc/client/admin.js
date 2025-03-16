import './general.css'
import './admin.css'
import * as general from './general.js'
import * as syntaxhi from './syntaxhi.js'
import { log } from './utils.js'

const loadingDialog = document.getElementById("loading-dialog");
const saveDialog = document.getElementById("save-dialog");
const saveLintsEl = document.getElementById("lints");
const saveDialogTitle = document.getElementById("save-dialog-title");
const saveOkBtn = document.getElementById("ok-btn");
const saveBtn = document.getElementById("save-btn");
const showlintsBtn = document.getElementById("show-lints-btn");
const ruleAddBtn = document.getElementById("rule-add-btn");
const ruleTemplate = document.getElementById("rule-template");
const rulesListEl = document.getElementById("rules-list");

const repRuleTemplate = document.getElementById("replace-rule-template");
const repRulesListEl = document.getElementById("replace-rules-list");
const repRuleAddBtn = document.getElementById("replace-rule-add-btn");

const REP_RULE="rep";
const MATCH_RULE="match";

const LINTS = JSON.parse(document.getElementById("jsonLints").textContent);
const RULES = JSON.parse(document.getElementById("jsonRuleset").textContent);
const TOKEN_INFO = JSON.parse(document.getElementById("tokenInfo").textContent);

let changes = false;

syntaxhi.setTokenInfo(TOKEN_INFO);

let str_to_token = {};
let token_to_str = {};
for (let token of TOKEN_INFO) {
  token_to_str[token[1]] = ["/"+token[0], token[2]];
  str_to_token[token[0]] = token[1];
}

function tokensToString(jsonTg) {
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
function stringToTokens(string) {
  let tokens = [];
  let escape = false;
  for (let char of string) {
    if (char == '/' && !escape) {
      escape=true;
      continue;
    }
    if (escape) {
      tokens.push(str_to_token[char]);
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

function toggleEnabledHTMLRule(rule) {
  enableHTMLRule(rule, rule.classList.contains("rule-disabled"));
}
function enableHTMLRule(rule, value) {
  let toggleBtn = rule.querySelector(".rule-disable-toggle");
  let ruleDelBtn = rule.querySelector(".rule-del-btn");

  if (value) {
    rule.classList.remove("rule-disabled");
    toggleBtn.innerText = "Disable rule";
    ruleDelBtn.disabled=true;
  } else {
    rule.classList.add("rule-disabled");
    toggleBtn.innerText = "Enable rule";
    ruleDelBtn.disabled=false;
  }
}

function markChanges() {
  changes = true;
}
function unmarkChanges() {
  changes = false;
}
window.addEventListener('beforeunload', (e) => {
  if (changes) {
    event.preventDefault();
  }
});

function createHTMLRule(jsonRule, type, anDelay=0) {

  let template;
  if (type == MATCH_RULE) {
    template = ruleTemplate;
  } else if (REP_RULE) {
    template = repRuleTemplate;
  }

  let ruleFrag = template.content.cloneNode(true);
  let rule = ruleFrag.querySelector(".rule");
  rule.style=`animation-delay:${anDelay}s`

  rule.addEventListener("input", (e) => { markChanges() } );

  rule.querySelector(".rule-disable-toggle").addEventListener("click", (e) => {
    toggleEnabledHTMLRule(rule);
    markChanges();
  });
  rule.querySelector(".rule-del-btn").addEventListener("click", (e) => {
    rule.classList.add("rule-deleted");
    markChanges();
  });
  enableHTMLRule(rule, jsonRule.enabled);

  if (type == MATCH_RULE) {
    let ruleInput = rule.querySelector(".rule-input");
    syntaxhi.createEditor(ruleInput);
    syntaxhi.setContent(ruleInput, tokensToString(jsonRule.tokens));

    for (let flag of jsonRule.flags) {
      rule.querySelector(`.rule-option[data-flagname="${flag}"]`).classList.add("checked");
    }

    rule.id = "matchrule-"+rulesListEl.childElementCount;
    rulesListEl.appendChild(rule);
  } else if (type == REP_RULE) {
    let matchInput = rule.querySelector(".match-input");
    syntaxhi.createEditor(matchInput);
    syntaxhi.setContent(matchInput, jsonRule.match_chars);

    let replaceInput = rule.querySelector(".replace-input");
    syntaxhi.createEditor(replaceInput);
    syntaxhi.setContent(replaceInput, tokensToString(jsonRule.replace_tg));

    rule.id = "reprule-"+repRulesListEl.childElementCount;
    repRulesListEl.appendChild(rule);
  }


  return rule;
}

function createHTMLLint(jsonLint, type, primaryLink=true) {
  let lint = document.createElement("div");
  lint.classList.add("lint");
  switch (jsonLint.importance) {
    case "Notify":
      lint.classList.add("lint-notify");
      break;
    case "Error":
      lint.classList.add("lint-error");
      break;
    default:
      break;
  }
  let message = document.createElement("span");
  message.innerText = jsonLint.message;
  lint.appendChild(message);

  let links = document.createElement("div");
  links.classList.add("lint-links");

  if (primaryLink) {
    let affectedRule = document.createElement("a");
    affectedRule.innerText="rule "+jsonLint.affected_rule;
    affectedRule.href=`#${type}rule-${jsonLint.affected_rule}`;
    links.appendChild(affectedRule);
  }

  if (jsonLint.second_affected_rule) {
    let affectedRule2 = document.createElement("a");
    affectedRule2.innerText="rule "+jsonLint.second_affected_rule;
    affectedRule2.href=`#${type}rule-${jsonLint.second_affected_rule}`;
    links.appendChild(affectedRule2);
  }
  lint.appendChild(links);

  return lint;
}

function createHTMLLintSet(lintset) {
  for (let i=0; i < repRulesListEl.childNodes.length; i++) {
    let lintsEl = repRulesListEl.childNodes[i].querySelector(".lints");
    lintsEl.innerHTML = "";

    if (lintset == null) { continue; }

    for (let lint of lintset.rep_lints) {
      if (lint.affected_rule == i){
        lintsEl.appendChild(createHTMLLint(lint, type=REP_RULE, primaryLink=false));
      }
    }
  }
  for (let i=0; i < rulesListEl.childNodes.length; i++) {
    let lintsEl = rulesListEl.childNodes[i].querySelector(".lints");
    lintsEl.innerHTML = "";

    if (lintset == null) { continue; }

    for (let lint of lintset.match_lints) {
      if (lint.affected_rule == i) {
        lintsEl.appendChild(createHTMLLint(lint, type=MATCH_RULE, primaryLink=false));
      }
    }
  }

  saveLintsEl.innerHTML = "";

  if (lintset == null) { return; }

  for (let lint of lintset.rep_lints) {
    saveLintsEl.appendChild(createHTMLLint(lint, type=REP_RULE));
  }
  for (let lint of lintset.match_lints) {
    saveLintsEl.appendChild(createHTMLLint(lint, type=MATCH_RULE));
  }

}

function prepHTMLRulesAndGenerateJson() {
  let repRulesJson = [];

  for (let i=0; i < repRulesListEl.childNodes.length; i++) {
    let htmlRule = repRulesListEl.childNodes[i];
    if (htmlRule.nodeName == "#text" || htmlRule.classList.contains("rule-deleted")) {
      htmlRule.remove();
      i--;
      continue;
    };
    htmlRule.querySelector(".lints").innerHTML= "";
    htmlRule.id = "reprule-"+i;
    let matchInput = htmlRule.querySelector(".match-input");
    let repInput = htmlRule.querySelector(".replace-input");
    repRulesJson.push({
      match_chars: syntaxhi.getContent(matchInput),
      replace_tg: stringToTokens(syntaxhi.getContent(repInput)),
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  let matchRulesJson = [];
  for (let i=0; i < rulesListEl.childNodes.length; i++) {
    let htmlRule = rulesListEl.childNodes[i];
    if (htmlRule.nodeName == "#text" || htmlRule.classList.contains("rule-deleted")) {
      htmlRule.remove();
      i--;
      continue;
    };
    htmlRule.querySelector(".lints").innerHTML= "";
    htmlRule.id = "matchrule-"+i;
    let tokenInput = htmlRule.querySelector(".rule-input");
    let flags = [];
    for (let htmlFlag of htmlRule.querySelectorAll(".rule-option")) {
      if (htmlFlag.classList.contains("checked")) {
        flags.push(htmlFlag.innerText);
      }
    }

    matchRulesJson.push({
      flags:flags,
      tokens: stringToTokens(syntaxhi.getContent(tokenInput)),
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  return {match_rules:matchRulesJson, rep_rules:repRulesJson };
}

async function postRuleset(rulesetJsonString) {
  const response = await fetch(ROOT_URL+"/admin/prof/ruleset", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body:rulesetJsonString,
  });
  if (response.status == 200) {
    let lints;
    try{
      lints = await response.json();
    }catch(e) {
      console.error("Failed to parse json in ok response: "+e);
    return [null, "Error while parsing json from server."];
    }
    return [lints, ""];
  } else if (response.status = 422) {
    let errJson;
    try {
      errJson = await response.json();
    } catch(e) {
      console.error("Failed to parse json in response: "+e);
      return [null, "Got "+response.status+" while connecting to server"];
    }
    return [errJson.lints, errJson.message];

  } else {
    return [null, "Got "+response.status+" while connecting to server"];
  }
}

async function saveChangesToServer() {
  loadingDialog.showModal();
  let json = prepHTMLRulesAndGenerateJson();

  let [lints, errMsg] = await postRuleset(JSON.stringify(json));

  if (errMsg != ""){
    saveDialogTitle.innerText = errMsg;
    saveDialogTitle.style = "color: var(--color-error)";
  }else {
    saveDialogTitle.innerText = "Rules have been saved succesfuly";
    saveDialogTitle.style = "";

    unmarkChanges();
  }
  createHTMLLintSet(lints);

  loadingDialog.close();
  saveDialog.showModal();

}

function createRulesFromEmbeddedJson() {
  repRulesListEl.innerHTML = "";
  for (let i=0; i < RULES.rep_rules.length; i++) {
    let rule = RULES.rep_rules[i];
    let delay = 0;
    if (i < 10) {
      delay = i*0.05
    }

   createHTMLRule(rule, REP_RULE, delay);
  }

  rulesListEl.innerHTML = "";
  for (let rule of RULES.match_rules) {
    createHTMLRule(rule, MATCH_RULE, 0)
  }

  createHTMLLintSet(LINTS);
}

saveOkBtn.addEventListener("click", (e)=>{
  saveDialog.close();
})


ruleAddBtn.addEventListener("click", (e)=> {
  createHTMLRule({enabled:true, tokens:[], flags:[] }, type=MATCH_RULE);
});

repRuleAddBtn.addEventListener("click", (e)=> {
  createHTMLRule({enabled:true, match_chars: "", replace_tg:[] }, type=REP_RULE);
})

saveBtn.addEventListener("click", saveChangesToServer);

window.addEventListener("keydown", (e) => {
  if (e.key === 's' && e.ctrlKey) {
    saveChangesToServer();
    e.preventDefault();
  }
  if (e.key === 'e' && e.ctrlKey) {
    saveDialog.showModal();
    e.preventDefault();
  }
})

showlintsBtn.addEventListener("click", (e) => {
  saveDialog.showModal();
});

unmarkChanges();
createRulesFromEmbeddedJson();
