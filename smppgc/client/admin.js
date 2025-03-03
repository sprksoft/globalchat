import './admin.css'
import './general.css'
import * as general from './general.js'
import * as utils from './utils.js'
import * as syntaxhi from './syntaxhi.js'

const saveBtn = document.getElementById("save-btn");
const ruleAddBtn = document.getElementById("rule-add-btn");
const ruleTemplate = document.getElementById("rule-template");
const rulesListEl = document.getElementById("rules-list");

const repRuleTemplate = document.getElementById("replace-rule-template");
const repRulesListEl = document.getElementById("replace-rules-list");
const repRuleAddBtn = document.getElementById("replace-rule-add-btn");

const RULES = JSON.parse(document.getElementById("jsonRules").textContent);
const TOKEN_INFO = JSON.parse(document.getElementById("tokenInfo").textContent);

syntaxhi.setTokenInfo(TOKEN_INFO);

let token_to_str = {};
for (let token of TOKEN_INFO) {
  token_to_str[token[1]] = ["/"+token[0], token[2]];
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
  console.log(string);
  return string;
}

function createHTMLMatchRule(jsonRule) {
  let ruleFrag = ruleTemplate.content.cloneNode(true);
  let rule = ruleFrag.querySelector(".rule");

  let ruleInput = rule.querySelector(".rule-input");
  syntaxhi.createEditor(ruleInput);
  syntaxhi.setContent(ruleInput, tokensToString(jsonRule.tokens));

  for (let flag of jsonRule.flags) {
    rule.querySelector(`.rule-option[data-flagname="${flag}"]`).classList.add("checked");
  }

  rule.querySelector(".rule-del").addEventListener("click", (e) => {
    rule.remove();
  });

  rulesListEl.appendChild(rule);
}

function createHTMLRepRule(jsonRule) {
  let ruleFrag = repRuleTemplate.content.cloneNode(true);
  let rule = ruleFrag.querySelector(".rule");

  let matchInput = rule.querySelector(".match-input");
  syntaxhi.createEditor(matchInput);
  syntaxhi.setContent(matchInput, jsonRule.match_chars);

  let replaceInput = rule.querySelector(".replace-input");
  syntaxhi.createEditor(replaceInput);
  syntaxhi.setContent(replaceInput, tokensToString(jsonRule.replace_tg));

  rule.querySelector(".rule-del").addEventListener("click", (e) => {
    rule.remove();
  });

  repRulesListEl.appendChild(rule);
}


ruleAddBtn.addEventListener("click", (e)=> {
  createHTMLMatchRule();
});

repRuleAddBtn.addEventListener("click", (e)=> {
  createHTMLRepRule();
})


saveBtn.addEventListener("click", (e) => {

});


for (let rule of RULES) {
  if (rule["Replace"])
    createHTMLRepRule(rule["Replace"]);

  if (rule["Match"])
    createHTMLMatchRule(rule["Match"]);
}
