import './admin.css'
import './general.css'
import * as general from './general.js'
import * as syntaxhi from './syntaxhi.js'
import { log } from './utils.js'

const loadingDialog = document.getElementById("loading-dialog");
const saveDialog = document.getElementById("save-dialog");
const saveBtn = document.getElementById("save-btn");
const revertBtn = document.getElementById("revert-btn");
const ruleAddBtn = document.getElementById("rule-add-btn");
const ruleTemplate = document.getElementById("rule-template");
const rulesListEl = document.getElementById("rules-list");

const repRuleTemplate = document.getElementById("replace-rule-template");
const repRulesListEl = document.getElementById("replace-rules-list");
const repRuleAddBtn = document.getElementById("replace-rule-add-btn");

const RULES = JSON.parse(document.getElementById("jsonRules").textContent);
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
  revertBtn.disabled=false;
  saveBtn.disabled=false;
}
function unmarkChanges() {
  changes = false;
  revertBtn.disabled=true;
  saveBtn.disabled=true;
}
window.addEventListener('beforeunload', (e) => {
  if (changes) {
    event.preventDefault();
  }
});

function createHTMLRule(jsonRule, anDelay=0) {
  let repRule = jsonRule["Replace"];
  let matchRule = jsonRule["Match"];

  let template;
  if (matchRule) {
    template = ruleTemplate;
  } else if (repRule) {
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

  if (matchRule) {
    let ruleInput = rule.querySelector(".rule-input");
    syntaxhi.createEditor(ruleInput);
    syntaxhi.setContent(ruleInput, tokensToString(matchRule.tokens));

    for (let flag of matchRule.flags) {
      rule.querySelector(`.rule-option[data-flagname="${flag}"]`).classList.add("checked");
    }

    rulesListEl.appendChild(rule);
  } else if (repRule) {
    let matchInput = rule.querySelector(".match-input");
    syntaxhi.createEditor(matchInput);
    syntaxhi.setContent(matchInput, repRule.match_chars);

    let replaceInput = rule.querySelector(".replace-input");
    syntaxhi.createEditor(replaceInput);
    syntaxhi.setContent(replaceInput, tokensToString(repRule.replace_tg));

    repRulesListEl.appendChild(rule);
  }


  return rule;
}

function generateJsonFromCurrentRules() {
  let rulesJson = [];

  for (let htmlRule of repRulesListEl.childNodes) {
    if (htmlRule.nodeName == "#text") {continue;};
    let matchInput = htmlRule.querySelector(".match-input");
    let repInput = htmlRule.querySelector(".replace-input");
    rulesJson.push({
      "Replace": {
        match_chars: syntaxhi.getContent(matchInput),
        replace_tg: stringToTokens(syntaxhi.getContent(repInput))
      },
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  for (let htmlRule of rulesListEl.childNodes) {
    if (htmlRule.nodeName == "#text") {continue;};
    let tokenInput = htmlRule.querySelector(".rule-input");
    let flags = [];
    for (let htmlFlag of htmlRule.querySelectorAll(".rule-option")) {
      if (htmlFlag.classList.contains("checked")) {
        flags.push(htmlFlag.innerText);
      }
    }

    rulesJson.push({
      "Match": {
        flags:flags,
        tokens: stringToTokens(syntaxhi.getContent(tokenInput))
      },
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  return rulesJson;
}

async function checkChanges() {
  loadingDialog.showModal();
  let json = generateJsonFromCurrentRules();
  if (JSON.stringify(RULES) == JSON.stringify(json)) {
    //TODO: show errors that we already got.
    loadingDialog.close();
    saveDialog.showModal();
    return;
  }

  const response = await fetch("/admin/prof/ruleset", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body:json,
  });

}


ruleAddBtn.addEventListener("click", (e)=> {
  createHTMLRule({"Match": {tokens:[], flags:[]}, enabled:true});
});

repRuleAddBtn.addEventListener("click", (e)=> {
  createHTMLRule({"Replace":{match_chars: "", replace_tg:[]}, enabled:true});
})

saveBtn.addEventListener("click", checkChanges);

revertBtn.addEventListener("click", (e) => {
  unmarkChanges();
  repRulesListEl.innerHTML="";
  rulesListEl.innerHTML="";
  createRulesFromEmbeddedJson();
});

function createRulesFromEmbeddedJson() {
  for (let i=0; i < RULES.length; i++) {
    let rule = RULES[i];
    let delay = 0;
    if (i < 10) {
      delay = i*0.05
    }
    createHTMLRule(rule, delay);
  }
}

createRulesFromEmbeddedJson();
