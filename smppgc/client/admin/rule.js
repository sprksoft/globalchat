import * as syntax from './syntax.js'

const ruleTemplate = document.getElementById("rule-template");
const repRuleTemplate = document.getElementById("replace-rule-template");

export const rulesListEl = document.getElementById("rules-list");
export const repRulesListEl = document.getElementById("replace-rules-list");

export const REP="rep";
export const MATCH="match";

export let rules = [];
export let repDeletions = [];
export let matchDeletions = [];

function onChanges(rule) {
  if (!rule.classList.contains("rule-changes")) {
    rule.classList.add("rule-changes");
    if (rule.dataset.origIndex) {
      if (rule.classList.contains("rep-rule")) {
        let origRule = rules.rep_rules[Number(rule.dataset.origIndex)];
        repDeletions.push(origRule);
      }else if (rule.classList.contains("match-rule")) {
        let origRule = rules.match_rules[Number(rule.dataset.origIndex)];
        matchDeletions.push(origRule);
      }
    }
  }
}

export function createHTMLRule(jsonRule, type, anDelay=0, origIndex=-1, insertTop=false, userCreated=false) {

  let template;
  if (type == MATCH) {
    template = ruleTemplate;
  } else if (type == REP) {
    template = repRuleTemplate;
  }else {
    console.error("Invalid rule type: '"+type+"'");
  }
  let ruleFrag = template.content.cloneNode(true);
  let rule = ruleFrag.querySelector(".rule");
  rule.style=`animation-delay:${anDelay}s`
  if (origIndex != -1) {
    rule.dataset.origIndex=origIndex;
  }

  rule.addEventListener("input", (e) => { onChanges(rule) } );

  rule.querySelector(".rule-disable-toggle").addEventListener("click", (e) => {
    toggleEnabledHTMLRule(rule);
    onChanges(rule);
  });
  rule.querySelector(".rule-del-btn").addEventListener("click", (e) => {
    rule.classList.add("rule-deleted");
    onChanges(rule);
  });
  enableHTMLRule(rule, jsonRule.enabled);

  if (userCreated) {
    rule.classList.add("rule-changes");
  }

  let matchInput = rule.querySelector(".match-input");
  let parent;
  if (type == MATCH) {
    rule.classList.add("match-rule");
    syntax.createEditor(matchInput);
    syntax.setContent(matchInput, syntax.tokensToString(jsonRule.tokens));

    for (let flag of jsonRule.flags) {
      rule.querySelector(`.rule-option[data-flagname="${flag}"]`).classList.add("checked");
    }
    rule.querySelector(".rule-options").addEventListener("click", (e)=> {onChanges(rule)});

    rule.id = "matchrule-"+rulesListEl.childElementCount;
    parent = rulesListEl;
  } else if (type == REP) {
    rule.classList.add("rep-rule");
    syntax.createEditor(matchInput);
    syntax.setContent(matchInput, jsonRule.match_chars);

    let replaceInput = rule.querySelector(".replace-input");
    syntax.createEditor(replaceInput);
    syntax.setContent(replaceInput, syntax.tokensToString(jsonRule.replace_tg));

    rule.id = "reprule-"+repRulesListEl.childElementCount;

    parent = repRulesListEl;
  }
  if (insertTop) {
    parent.insertBefore(rule, parent.firstChild);
  } else {
    parent.appendChild(rule);
  }

  if (userCreated) {
    console.log("focus input");
    syntax.focus(matchInput);
  }


  return rule;
}

export function getJson(htmlRule) {
  let matchInput = htmlRule.querySelector(".match-input");
  let repInput = htmlRule.querySelector(".replace-input");
  let enabled = !htmlRule.classList.contains("rule-disabled");
  if (repInput) {
    return {
      match_chars: syntax.getContent(matchInput),
      replace_tg: syntax.stringToTokens(syntax.getContent(repInput)),
      enabled:enabled
    };
  } else {
    let flags = [];
    for (let htmlFlag of htmlRule.querySelectorAll(".rule-option")) {
      if (htmlFlag.classList.contains("checked")) {
        flags.push(htmlFlag.innerText);
      }
    }
    return {
      tokens: syntax.stringToTokens(syntax.getContent(matchInput)),
      flags:flags,
      enabled:enabled
    };
  }
}

export function toggleEnabledHTMLRule(rule) {
  enableHTMLRule(rule, rule.classList.contains("rule-disabled"));
}
export function enableHTMLRule(rule, value) {
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

export function setRules(ruleset) {
  repDeletions = [];
  matchDeletions = [];
  repRulesListEl.innerHTML = "";
  for (let i=0; i < ruleset.rep_rules.length; i++) {
    let rule = ruleset.rep_rules[i];
    let delay = 0;
    if (i < 10) {
      delay = i*0.05
    }
   createHTMLRule(rule, REP, anDelay=delay,origIndex=i, insertTop=false, userCreated=false);
  }

  rulesListEl.innerHTML = "";
  for (let i=0; i < ruleset.match_rules.length; i++) {
    let rule = ruleset.match_rules[i];
    createHTMLRule(rule, MATCH, anDelay=0,origIndex=i, insertTop=false, userCreated=false);
  }

  rules = ruleset;
  localStorage.setItem("last_rule_update_time", new Date().getTime());
}
