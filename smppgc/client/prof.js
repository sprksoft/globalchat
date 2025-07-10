import * as common from './common/text.js'
import { log, getCSRFToken } from './common/utils.js'
import * as syntax from './prof/syntax.js'
import * as rule from './prof/rule.js'

import './common/common.css'
import './common/buttons.css'
import './prof/css/prof.css'

const loadingDialog = document.getElementById("loading-dialog");
const saveDialog = document.getElementById("save-dialog");
const lintsEl = document.getElementById("lints");
const saveDialogTitle = document.getElementById("save-dialog-title");
const saveBtn = document.getElementById("save-btn");
const lintsDialog = document.getElementById("lints-dialog");
const showlintsBtn = document.getElementById("show-lints-btn");
const showlintsBtn2 = document.getElementById("show-lints-btn2");

const ruleAddBtn = document.getElementById("rule-add-btn");
const repRuleAddBtn = document.getElementById("replace-rule-add-btn");

const TOKEN_INFO = JSON.parse(document.getElementById("tokenInfo").textContent);
syntax.setTokenInfo(TOKEN_INFO);

let currentLints;

window.addEventListener('beforeunload', (e) => {
  if (hasUnsavedChanges()) {
    event.preventDefault();
  }
});

function hasUnsavedChanges() {
  return !(document.querySelectorAll(".rule-changes").length == 0 && rule.matchDeletions.length == 0 && rule.repDeletions.length == 0);
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

function setLints(lintset) {
  for (let i=0; i < rule.repRulesListEl.childNodes.length; i++) {
    let lintsEl = rule.repRulesListEl.childNodes[i].querySelector(".lints");
    if (lintsEl == null) { continue; }
    lintsEl.innerHTML = "";

    for (let lint of lintset.rep_lints) {
      if (lint.affected_rule == i) {
        lintsEl.appendChild(createHTMLLint(lint, type=rule.REP, primaryLink=false));
      }
    }
  }
  for (let i=0; i < rule.rulesListEl.childNodes.length; i++) {
    let lintsEl = rule.rulesListEl.childNodes[i].querySelector(".lints");
    if (lintsEl == null) { continue; }
    lintsEl.innerHTML = "";

    for (let lint of lintset.match_lints) {
      if (lint.affected_rule == i) {
        lintsEl.appendChild(createHTMLLint(lint, type=rule.MATCH, primaryLink=false));
      }
    }
  }

  lintsEl.innerHTML = "";
  currentLintset = lintset;
  if (lintset.rep_lints.length == 0 && lintset.match_lints.length == 0){
    showlintsBtn.disabled=true;
    showlintsBtn2.disabled=true;
    return;
  }
  showlintsBtn.disabled=false;
  showlintsBtn2.disabled=false;

  for (let lint of lintset.rep_lints) {
    lintsEl.appendChild(createHTMLLint(lint, type=rule.REP));
  }
  for (let lint of lintset.match_lints) {
    lintsEl.appendChild(createHTMLLint(lint, type=rule.MATCH));
  }
}



function prepHTMLRulesAndGenerateJson() {
  let repRulesJson = [];

  for (let i=0; i < repRulesListEl.childNodes.length; i++) {
    let htmlRule = repRulesListEl.childNodes[i];
    if (!htmlRule.classList.contains("rule")) {
      continue;
    }
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
      match_chars: syntax.getContent(matchInput),
      replace_tg: syntax.stringToTokens(syntax.getContent(repInput)),
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  let matchRulesJson = [];
  for (let i=0; i < rulesListEl.childNodes.length; i++) {
    let htmlRule = rulesListEl.childNodes[i];
    if (!htmlRule.classList.contains("rule")){
      continue;
    }
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
      tokens: syntax.stringToTokens(syntax.getContent(tokenInput)),
      enabled:!htmlRule.classList.contains("rule-disabled")
    });
  }

  return {match_rules:matchRulesJson, rep_rules:repRulesJson };
}

async function apiCall(changes) {
  let response;
  try {
    response = await fetch("/api/prof/ruleset", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-CSRF-Protect": getCSRFToken(),
      },
      body:JSON.stringify(changes),
    });

    if (response.status !== 200) {
      return "Got "+response.status+" from server";
    }
  } catch(e) {
    return e.toString();
  }
  let responseJson;
  try{
    responseJson = await response.json();
  }catch(e) {
    console.error("Failed to parse json response: "+e);
    return "Failed to pares json from server.";
  }
  return responseJson;
}

/// Push changes to the server and apply new rules returned from the server
async function syncChanges(loadOnly=false) {
  loadingDialog.showModal();
  let changes = {
    rep_additions: [],
    match_additions: [],
    rep_deletions: [],
    match_deletions: []
  }
  if (!loadOnly) {
    changes.rep_deletions = rule.repDeletions;
    changes.match_deletions = rule.matchDeletions;
    for (let htmlRule of document.querySelectorAll(".rule-changes.rep-rule:not(.rule-deleted)")) {
      changes.rep_additions.push(rule.getJson(htmlRule));
    }
    for (let htmlRule of document.querySelectorAll(".rule-changes.match-rule:not(.rule-deleted)")) {
      changes.match_additions.push(rule.getJson(htmlRule));
    }
  }

  let response;
  if (document.querySelectorAll(".rule:not(.rule-deleted) .rule-input[data-syntaxerror=true]").length != 0) {
    response = "Ruleset contains syntax errors";
  }else {
    response = await apiCall(changes);
  }

  let titleEl = document.getElementById("save-dialog-title");
  let messageEl = document.getElementById("save-dialog-message");
  let error = false;
  if (typeof response == "string") {
    error = true;
    if (loadOnly) {
      titleEl.innerText="Failed to load rules";
    }else {
      titleEl.innerText="Failed to save rules";
    }
    titleEl.classList.add("save-error");
    messageEl.innerText=response;
  }else {
    titleEl.innerText="Saved succesfuly";
    titleEl.classList.remove("save-error");
    messageEl.innerText=response.lints.rep_lints.length == 0 && response.lints.match_lints.length == 0 ? "" : "But there are problems.";
    rule.setRules(response.rules);
    setLints(response.lints);
  }

  loadingDialog.close();
  if (error || !loadOnly) {
    saveDialog.showModal();
  }
}


ruleAddBtn.addEventListener("click", (e)=> {
  rule.createHTMLRule({enabled:true, tokens:[], flags:[] }, type=rule.MATCH, anDelay=0, origIndex=-1, insertTop=true, userCreated=true);
});

repRuleAddBtn.addEventListener("click", (e)=> {
  rule.createHTMLRule({enabled:true, match_chars: "", replace_tg:[] }, type=rule.REP, anDelay=0, origIndex=-1, insertTop=true, userCreated=true);
})

saveBtn.addEventListener("click", (e) => { syncChanges() });

window.addEventListener("keydown", (e) => {
  if (e.key === 's' && e.ctrlKey) {
    syncChanges();
    e.preventDefault();
  }
  if (e.key === 'e' && e.ctrlKey) {
    lintsDialog.showModal();
    e.preventDefault();
  }
})

showlintsBtn.addEventListener("click", (e) => {
  lintsDialog.showModal();
});
showlintsBtn2.addEventListener("click", (e) => {
  lintsDialog.showModal();
});

syncChanges(loadOnly=true);
