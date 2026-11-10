// The script of a page in a sticky. It runs once, after the rows of the items are in place.
// The app passes its values in the data attributes of the body:
//   data-pages: the address that all pages live under
//   data-zoom:  the zoom factor, where 1 is 100%
// The script tells the app what the user did with window.chrome.webview.postMessage.
// The app reads these messages in WebMessage.cs. Keep the names the same in both files.
(function(){
  document.querySelectorAll('.task-checkbox[data-running]').forEach(function(el){ el.indeterminate=true; });
  document.querySelectorAll('[data-ast]').forEach(function(el){
    try { el.innerHTML=FmAst2Html.renderToHtml(el.getAttribute('data-ast')); } catch(e){}
  });
  // An image is written with a path relative to its page, such as ../assets/pic.png.
  // This document has no address of its own, so the path is resolved against the pages.
  var pages=document.body.dataset.pages;
  document.querySelectorAll('[data-ast] [src]').forEach(function(el){
    try { el.setAttribute('src',new URL(el.getAttribute('src'),pages).href); } catch(e){}
  });
  var z=Number(document.body.dataset.zoom)*100;
  function apply(){ document.body.style.zoom=z+'%'; window.chrome.webview.postMessage({zoom:z/100}); }
  document.addEventListener('wheel',function(e){
    if(!e.ctrlKey)return;
    e.preventDefault();
    z=Math.min(300,Math.max(30,z+(e.deltaY<0?10:-10)));
    apply();
  },{passive:false});
  document.addEventListener('keydown',function(e){
    if(!e.ctrlKey)return;
    if(e.key==='+'||e.key==='='){ e.preventDefault(); z=Math.min(300,z+10); apply(); }
    else if(e.key==='-'){ e.preventDefault(); z=Math.max(30,z-10); apply(); }
    else if(e.key==='0'){ e.preventDefault(); z=100; apply(); }
  });
  document.addEventListener('click',function(e){
    var t=e.target.closest('.collapse-toggle');
    if(t){ window.chrome.webview.postMessage({toggleCollapse:t.getAttribute('data-item-id')}); }
    // The href of a wikilink or a hashtag is a page name, not a URL. Following it would
    // replace the note with an error page. The app opens the page in the web app instead.
    var w=e.target.closest('a[data-wikilink]');
    var h=e.target.closest('a[data-hashtag],a.hashtag');
    if(w||h){
      e.preventDefault();
      var name=(w||h).getAttribute('href');
      if(name){ window.chrome.webview.postMessage(w?{openWikilink:name}:{openHashtag:name}); }
    }
  });
})();
