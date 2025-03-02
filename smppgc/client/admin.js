import './admin.css'
import './general.css'

let ruleTemplate = document.getElementById("rule-template");
let rulesListEl = document.getElementById("rules-list");
let ruleAddEl = document.getElementById("rule-add");

function ruleOptClick(e) {
  if (e.target.classList.contains("checked")){
    e.target.classList.remove("checked");
  }else{
    e.target.classList.add("checked");
  }
}


function createHTMLRule() {
  let ruleFrag = ruleTemplate.content.cloneNode(true);
  let rule = ruleFrag.querySelector(".rule");
  rule.querySelector(".rule-options");

  rule.querySelector(".rule-del").addEventListener("click", (e) => {
    console.log("deleting rule");
    rule.remove();
  });

  rulesListEl.appendChild(rule);
}

ruleAddEl.addEventListener("click", (e)=>{
  createHTMLRule();
});


