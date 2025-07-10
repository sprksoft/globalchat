document.addEventListener("input", (e) =>{
  if (e.target.dataset.plaintext == "true" && e.dataTransfer !== null) {
    let input = e.target;
    for (let i =0; i < input.childNodes.length; i++){
      let child = input.childNodes[i];
      if (child.nodeName !== "#text"){
        let text_node = document.createTextNode(child.innerText);
        input.insertBefore(text_node, child);
        child.remove();
        i--;
      }
    }
  }
});
