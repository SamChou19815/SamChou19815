// @generated
/* eslint-disable */
// prettier-ignore
(() => {
var Bt=Object.create;var F=Object.defineProperty;var Ut=Object.getOwnPropertyDescriptor;var Gt=Object.getOwnPropertyNames;var Yt=Object.getPrototypeOf,Jt=Object.prototype.hasOwnProperty;var ut=t=>F(t,"__esModule",{value:!0});var qt=(t,e)=>()=>(e||t((e={exports:{}}).exports,e),e.exports),tt=(t,e)=>{ut(t);for(var r in e)F(t,r,{get:e[r],enumerable:!0})},zt=(t,e,r)=>{if(e&&typeof e=="object"||typeof e=="function")for(let n of Gt(e))!Jt.call(t,n)&&n!=="default"&&F(t,n,{get:()=>e[n],enumerable:!(r=Ut(e,n))||r.enumerable});return t},f=t=>zt(ut(F(t!=null?Bt(Yt(t)):{},"default",t&&t.__esModule&&"default"in t?{get:()=>t.default,enumerable:!0}:{value:t,enumerable:!0})),t);var xt=qt(nt=>{function T(t,e){if(typeof t=="string")return t;if(t){let r,n;if(Array.isArray(t)){for(r=0;r<t.length;r++)if(n=T(t[r],e))return n}else for(r in t)if(e.has(r))return T(t[r],e)}}function x(t,e,r){throw new Error(r?`No known conditions for "${e}" entry in "${t}" package`:`Missing "${e}" export in "${t}" package`)}function wt(t,e){return e===t?".":e[0]==="."?e:e.replace(new RegExp("^"+t+"/"),"./")}function Zt(t,e=".",r={}){let{name:n,exports:o}=t;if(o){let{browser:i,require:a,conditions:s=[]}=r,l=wt(n,e);if(l[0]!=="."&&(l="./"+l),typeof o=="string")return l==="."?o:x(n,l);let p=new Set(["default",...s]);p.add(a?"require":"import"),p.add(i?"browser":"node");let c,u,d=!1;for(c in o){d=c[0]!==".";break}if(d)return l==="."?T(o,p)||x(n,l,1):x(n,l);if(u=o[l])return T(u,p)||x(n,l,1);for(c in o){if(u=c[c.length-1],u==="/"&&l.startsWith(c))return(u=T(o[c],p))?u+l.substring(c.length):x(n,l,1);if(u==="*"&&l.startsWith(c.slice(0,-1))&&l.substring(c.length-1).length>0)return(u=T(o[c],p))?u.replace("*",l.substring(c.length-1)):x(n,l,1)}return x(n,l)}}function te(t,e={}){let r=0,n,o=e.browser,i=e.fields||["module","main"];for(o&&!i.includes("browser")&&i.unshift("browser");r<i.length;r++)if(n=t[i[r]]){if(typeof n!="string")if(typeof n=="object"&&i[r]=="browser"){if(typeof o=="string"&&(n=n[o=wt(t.name,o)],n==null))return o}else continue;return typeof n=="string"?"./"+n.replace(/^\.?\//,""):n}}nt.legacy=te;nt.resolve=Zt});tt(exports,{constants:()=>it,default:()=>Ot,utils:()=>Te});var v=t=>process.stderr.isTTY?e=>`[${t}m${e}[0m`:e=>e,E=v(31),R=v(32),ct=v(33),j=v(34),Me=v(35),Ae=v(36);var h=f(require("path")),at=f(require("esbuild"));var _t=f(require("module")),Y=f(require("path")),kt=f(require("sass"));var rt={};tt(rt,{copyDirectoryContent:()=>I,copyFile:()=>_,emptyDirectory:()=>mt,ensureDirectory:()=>P,exists:()=>dt,isDirectory:()=>N,readDirectory:()=>et,readFile:()=>O,remove:()=>W,writeFile:()=>w});var m=f(require("fs")),y=f(require("path")),$=(t,e)=>r=>r?e(r):t(),D=t=>new Promise((e,r)=>(0,m.readdir)(t,(n,o)=>n?r(n):e(o))),pt=async t=>Promise.all((await D(t)).flatMap(async e=>{let r=(0,y.join)(t,e);return await N(r)?[r,...await pt(r)]:[r]})).then(e=>e.flat()),I=async(t,e)=>{await P(e),await mt(e),await Promise.all((await D(t)).map(async r=>{let n=(0,y.join)(t,r),o=(0,y.join)(e,r);await N(n)?await I(n,o):await _(n,o)}))},_=(t,e)=>new Promise((r,n)=>(0,m.copyFile)(t,e,$(r,n))),mt=async t=>{let e=await D(t);await Promise.all(e.map(r=>W((0,y.join)(t,r))))},P=t=>new Promise((e,r)=>(0,m.mkdir)(t,{recursive:!0},$(e,r))),dt=t=>new Promise(e=>(0,m.access)(t,void 0,r=>e(r==null))),N=t=>new Promise((e,r)=>(0,m.lstat)(t,(n,o)=>n?r(n):e(o.isDirectory()))),et=async(t,e)=>e?await dt(t)?(await pt(t)).map(r=>(0,y.relative)(t,r)).sort((r,n)=>r.localeCompare(n)):[]:D(t),O=t=>new Promise((e,r)=>(0,m.readFile)(t,(n,o)=>n?r(n):e(o.toString()))),W=t=>new Promise((e,r)=>{m.readFile!=null?(0,m.rm)(t,{recursive:!0,force:!0},$(e,r)):N(t).then(n=>n?(0,m.rmdir)(t,{recursive:!0},$(e,r)):(0,m.unlink)(t,$(e,r))).catch(n=>r(n))}),w=(t,e)=>new Promise((r,n)=>(0,m.writeFile)(t,e,$(r,n)));var bt=f(require("@mdx-js/mdx")),yt=f(require("remark-slug"));var ft=({level:t,label:e})=>`${"#".repeat(t)} ${e}`,Xt=t=>{let e=[],r=!1;return t.split(`
`).filter(o=>{if(o.startsWith("```"))r=!r;else return!r}).forEach(o=>{let i=o.trim();if(!i.startsWith("#"))return;let a=0;for(;i[a]==="#";)a+=1;if(a>6)throw new Error(`Invalid Header: '${i}'`);e.push({level:a,label:i.substring(a).trim()})}),e},gt=t=>{let e=Xt(t);if(e[0]==null)throw new Error("Lacking title.");if(e[0].level!==1)throw new Error(`First heading must be h1, found: ${ft(e[0])}`);if(e.filter(r=>r.level===1).length>1)throw new Error("More than one h1.");return e},ht=(t,e)=>{let r=t[e];if(r==null)throw new Error;let n=[],o=e+1;for(;o<t.length;){let{element:i,level:a,finishedIndex:s}=ht(t,o);if(a<=r.level)break;if(a>r.level+1){let l=ft({level:a,label:i.label});throw new Error(`Invalid header: ${l}. Expected Level: ${r.level+1}`)}o=s,n.push(i)}return{element:{label:r.label,children:n},level:r.level,finishedIndex:o}},Kt=t=>{let e=gt(t);return ht(e,0).element},Pt=t=>{let e=gt(t)[0];if(e==null)throw new Error;return e.label},B=Kt;var Qt=async(t,e)=>{let r=t.trim().split(`
`).slice(1),n;if(e){let o=r.findIndex(i=>i.trimStart().startsWith("<!--")&&i.includes("truncate"));n=r.slice(0,o).join(`
`)}else n=r.join(`
`);return n=n.trim(),`import React from 'react';
import mdx from 'esbuild-scripts/__internal-components__/mdx';
${await(0,bt.default)(n,{remarkPlugins:[yt.default]})}
MDXContent.truncated = ${e};
MDXContent.toc = ${JSON.stringify(B(t),void 0,2)};
MDXContent.additionalProperties = typeof additionalProperties === 'undefined' ? undefined : additionalProperties;
`},k=Qt;var vt=f(require("fs"));var U=f(require("fs")),g=f(require("path")),G=f(xt()),ee=[".tsx",".ts",".jsx",".mjs",".cjs",".js",".json"],re=/^(\/|\.{1,2}(\/|$))/,ne=/^\.{0,2}\//,oe=(t,e,r)=>{let n=(0,G.resolve)(t,e,{browser:r,require:!r});if(n!=null)return n;let o=(0,G.legacy)(t,r?{browser:r}:{browser:!1,fields:["main","module"]});return typeof o=="string"?e==="."?o:null:o&&(o[e]||o[`${e}.js`]||o[`./${e}`]||o[`./${e}.js`])||null},ie=(t,e)=>{let r=(0,g.normalize)(t),n=(0,g.normalize)(e);return r===n?".":(r.endsWith(g.sep)||(r=r+g.sep),n.startsWith(r)?n.slice(r.length):null)},se=(t,e,r)=>{let n=e.findPackageLocator((0,g.join)(t,"internal.js"));if(n==null)throw new Error;let{packageLocation:o}=e.getPackageInformation(n),i=(0,g.join)(o,"package.json");if(!U.existsSync(i))return null;let a=JSON.parse(U.readFileSync(i,"utf8")),s=ie(o,t);if(s==null)throw new Error;ne.test(s)||(s=`./${s}`),s=(0,g.normalize)(s);let l=oe(a,s,r);return l!=null?(0,g.join)(o,l):null},ae=(t,e,r,n)=>{if(re.test(t))return e;let o=se(e,r,n);return o?(0,g.normalize)(o):e},le=(t,e,r,n)=>{let o=r.resolveToUnqualified(t,e);return o==null?null:r.resolveUnqualified(ae(t,o,r,n),{extensions:ee})},St=le;var Et=/()/,ue=()=>({name:"esbuild-scripts-esbuild-plugin-pnp",setup(t){let{findPnpApi:e}=require("module");if(typeof e=="undefined")return;let r=process.cwd();t.onResolve({filter:Et},async n=>{let o=n.importer?n.importer:`${r}/`,i=e(o);if(!i)return;let a=n.path.lastIndexOf("?"),s=a===-1?n.path:n.path.substring(0,a),l=St(s,o,i,t.initialOptions.platform!=="node");return l==null?{external:!0}:{namespace:"pnp",path:`${l}${a===-1?"":n.path.substring(a)}`}}),t.onLoad!==null&&t.onLoad({filter:Et},async n=>({contents:await vt.promises.readFile(n.path,"utf8"),loader:"default"}))}}),Rt=ue;var $t=/^esbuild-scripts-internal\/virtual\//,ce=t=>({name:"VirtualPathResolvePlugin",setup(e){e.onResolve({filter:$t},r=>({path:r.path,namespace:"virtual-path"})),e.onLoad({filter:$t,namespace:"virtual-path"},r=>({contents:t[r.path],loader:"jsx"}))}}),Tt=ce;var pe={name:"WebAppResolvePlugin",setup(t){t.onResolve({filter:/^data:/},()=>({external:!0}))}},me={name:"sass",setup(t){t.onResolve({filter:/.\.(scss|sass)$/},async e=>e.path.startsWith(".")?{path:(0,Y.resolve)((0,Y.dirname)(e.importer),e.path)}:{path:(0,_t.createRequire)(e.importer).resolve(e.path)}),t.onLoad({filter:/.\.(scss|sass)$/},async e=>{let{css:r}=await new Promise((n,o)=>{(0,kt.render)({file:e.path},(i,a)=>{i?o(i):n(a)})});return{contents:r.toString(),loader:"css",watchFiles:[e.path]}})}},de={name:"mdx",setup(t){t.onLoad({filter:/\.md\?truncated=true$/},async e=>({contents:await k(await O(e.path.substring(0,e.path.lastIndexOf("?"))),!0),loader:"jsx"})),t.onLoad({filter:/\.md$/},async e=>({contents:await k(await O(e.path),!1),loader:"jsx"}))}},fe=t=>[pe,Tt(t),me,de,Rt()],Mt=fe;var ge=({virtualPathMappings:t,isServer:e=!1,isProd:r=!1})=>({define:{__dirname:'""',__SERVER__:String(e),"process.env.NODE_ENV":r?'"production"':'"development"'},bundle:!0,minify:!1,legalComments:"linked",platform:"browser",target:"es2019",logLevel:"error",plugins:Mt(t)}),M=ge;var it={};tt(it,{BUILD_PATH:()=>z,PAGES_PATH:()=>b,SSR_CSS_PATH:()=>ot,SSR_JS_PATH:()=>q,TEMPLATE_PATH:()=>J,VIRTUAL_GENERATED_ENTRY_POINT_PATH_PREFIX:()=>V,VIRTUAL_PATH_PREFIX:()=>H,VIRTUAL_SERVER_ENTRY_PATH:()=>L});var A=f(require("path")),J=(0,A.join)(__dirname,"templates"),b=(0,A.join)("src","pages"),q=(0,A.join)("build","__ssr.jsx"),ot=(0,A.join)("build","__ssr.css"),z="build",H="esbuild-scripts-internal/virtual/",V=`${H}__generated-entry-point__/`,L=`${V}__server__.jsx`;var X=f(require("path"));var At="// @generated",st=t=>t.endsWith("index")?t.substring(0,t.length-(t.endsWith("/index")?6:5)):t,Ht=(t,e,r)=>r?`${t}/src/pages/${e}`:`${H}${e}`,Vt=(t,e,r,n,o)=>{let i=n.filter(u=>u!==e),a=o.filter(u=>u!==e),s=(u,d)=>Ht(t,u,d),l=[...i.map((u,d)=>`const RealComponent${d} = lazy(() => import('${s(u,!0)}'));`),...a.map((u,d)=>`const VirtualComponent${d} = lazy(() => import('${s(u,!1)}'));`)].join(`
`),p=s(e,r),c=[...i.map((u,d)=>`        <Route exact path="/${st(u)}"><Suspense fallback={null}><RealComponent${d} /></Suspense></Route>`),...a.map((u,d)=>`        <Route exact path="/${st(u)}"><Suspense fallback={null}><VirtualComponent${d} /></Suspense></Route>`)].join(`
`);return`${At}
import React,{Suspense,lazy} from 'react';
import {hydrate,render} from 'react-dom';
import {BrowserRouter,Route,Switch} from 'esbuild-scripts/__internal-components__/react-router';
import Document from '${t}/src/pages/_document';
import Page from '${p}';
${l}
const element = (
  <BrowserRouter>
    <Document>
      <Switch>
        <Route exact path="/${st(e)}"><Page /></Route>
${c}
      </Switch>
    </Document>
  </BrowserRouter>
);
const rootElement = document.getElementById('root');
if (rootElement.hasChildNodes()) hydrate(element, rootElement); else render(element, rootElement);
`},he=(t,e,r)=>{let n=(a,s)=>Ht(t,a,s),o=[...e.map((a,s)=>`import RealPage${s} from '${n(a,!0)}';`),...r.map((a,s)=>`import VirtualPage${s} from '${n(a,!1)}';`)].join(`
`),i=[...e.map((a,s)=>`'${a}': RealPage${s}`),...r.map((a,s)=>`'${a}': VirtualPage${s}`)].join(", ");return`${At}
import React from 'react';
import {renderToString} from 'react-dom/server';
import Helmet from 'esbuild-scripts/components/Head';
import {StaticRouter} from 'esbuild-scripts/__internal-components__/react-router';
import Document from '${t}/src/pages/_document';
${o}
const map = { ${i} };
module.exports = (path) => ({
  divHTML: renderToString(
    <StaticRouter location={'/'+path}>
      <Document>{React.createElement(map[path])}</Document>
    </StaticRouter>
  ),
  noJS: map[path].noJS,
  helmet: Helmet.renderStatic(),
});
`},Pe=async()=>(await P(b),(await et(b,!0)).map(t=>{switch((0,X.extname)(t)){case".js":case".jsx":case".ts":case".tsx":break;default:return null}return t.substring(0,t.lastIndexOf("."))}).filter(t=>t!=null&&!t.startsWith("_document"))),C=t=>Object.fromEntries(Object.entries(t).map(([e,r])=>[`${H}${e}`,r])),K=async t=>{let e=(0,X.resolve)("."),r=await Pe(),n=Object.fromEntries([...r.map(o=>[`${V}${o}.jsx`,Vt(e,o,!0,r,t)]),...t.map(o=>[`${V}${o}.jsx`,Vt(e,o,!1,r,t)])]);return n[L]=he(e,r,t),{entryPointsWithoutExtension:r,entryPointVirtualFiles:n}};var be=(t,e)=>e?`<script type="module" src="/${t}"><\/script>`:`<script src="/${t}"><\/script>`,ye=(t,e)=>`<link rel="${e?"modulepreload":"preload"}" href="/${t}" />`,we=(t,e,r)=>{let n=[],o=[];t.forEach(s=>{!r&&s.endsWith("js")?n.push(s):s.endsWith("css")&&o.push(s)});let i=o.map(s=>`<link rel="stylesheet" href="/${s}" />`).join("")+n.map(s=>ye(s,e)).join(""),a=n.map(s=>be(s,e)).join("");return{headLinks:i,bodyScriptLinks:a}},Lt=(t,e)=>e==null?`<head>${t}</head>`:`<head>${[e.meta.toString(),e.title.toString(),e.link.toString(),e.script.toString(),t].join("")}</head>`,xe=(t,e,r)=>{let{headLinks:n,bodyScriptLinks:o}=we(e,r,t==null?void 0:t.noJS);if(t==null){let p=Lt(n),c=`<body><div id="root"></div>${o}</body>`;return`<!DOCTYPE html><html>${p}${c}</html>`}let{divHTML:i,helmet:a}=t,s=Lt(n,a),l=`<body><div id="root">${i}</div>${o}</body>`;return`<!DOCTYPE html><html ${a.htmlAttributes.toString()}>${s}${l}</html>`},Q=xe;var Ct=async(t,e)=>{let r={...t,...C(e)},{outputFiles:n}=await(0,at.build)({...M({virtualPathMappings:r,isProd:!0}),entryPoints:Object.keys(t),assetNames:"assets/[name]-[hash]",chunkNames:"chunks/[name]-[hash]",entryNames:"[dir]/[name]-[hash]",minify:!0,format:"esm",splitting:!0,write:!1,outdir:"build"}),o=(0,h.resolve)("build");return await Promise.all(n.map(async i=>{await P((0,h.dirname)(i.path)),await w(i.path,i.contents)})),n.map(({path:i})=>(0,h.relative)(o,i))},Se=async(t,e)=>{await(0,at.build)({...M({virtualPathMappings:{...t,...C(e)},isServer:!0,isProd:!0}),entryPoints:[L],platform:"node",format:"cjs",legalComments:"none",outfile:q});try{return require((0,h.resolve)(q))}catch(r){return console.error(E("Unable to perform server side rendering since the server bundle is not correctly generated.")),console.error(E(r)),null}finally{await W(ot)}},ve=async(t,e)=>{let r=new Date().getTime();console.error(ct("[i] Bundling..."));let{entryPointsWithoutExtension:n,entryPointVirtualFiles:o}=await K(Object.keys(t));await I("public","build");let i,a;if(e){if([i,a]=await Promise.all([Ct(o,t),Se(o,t)]),a==null)return!1}else i=await Ct(o,t),a=null;let s=[...n,...Object.keys(t)].map(p=>{let c=i.filter(d=>d.startsWith("chunk")||d.startsWith(p)),u=Q(a==null?void 0:a(p),c,!0);return{entryPoint:p,html:u}});await Promise.all(s.map(async({entryPoint:p,html:c})=>{let u;p.endsWith("index")?u=(0,h.join)(z,`${p}.html`):u=(0,h.join)(z,p,"index.html"),await P((0,h.dirname)(u)),await w(u,c)}));let l=new Date().getTime()-r;return console.error(`\u26A1 ${R(`Build success in ${l}ms.`)}`),!0},lt=ve;var S=f(require("path"));var Ee=async()=>{await P(b),await Promise.all([_((0,S.join)(J,"_document.tsx"),(0,S.join)(b,"_document.tsx")),_((0,S.join)(J,"index.tsx"),(0,S.join)(b,"index.tsx")),w((0,S.join)(b,"index.css"),""),w((0,S.join)("src","types.d.ts"),`/// <reference types="esbuild-scripts" />
`),P("public")]),console.error(R("esbuild-scripts app initialized."))},Ft=Ee;var Z=f(require("http")),jt=f(require("esbuild"));var Re=(t,e)=>{if(e==null||!e.startsWith("/"))return;let r=e.substring(1);return t.find(n=>n.endsWith("index")?[n,n.substring(0,n.length-5)].includes(r):n===r)},Dt=t=>Q(void 0,[`${t}.js`,`${t}.css`],!1),$e=async t=>{let{entryPointsWithoutExtension:e,entryPointVirtualFiles:r}=await K(Object.keys(t)),n={...r,...C(t)},o=[...e,...Object.keys(t)],i=await(0,jt.serve)({servedir:"public",port:19815},{...M({virtualPathMappings:n}),entryPoints:Object.keys(r),sourcemap:"inline",outdir:"public"}),a=(0,Z.createServer)((s,l)=>{let p=Re(o,s.url);if(p!=null){l.writeHead(200,{"Content-Type":"text/html"}),l.end(Dt(p));return}let c={hostname:i.host,port:i.port,path:s.url,method:s.method,headers:s.headers},u=(0,Z.request)(c,d=>{if(d.statusCode===404){l.writeHead(200,{"Content-Type":"text/html"}),l.end(Dt("index"));return}l.writeHead(d.statusCode||200,d.headers),d.pipe(l,{end:!0})});s.pipe(u,{end:!0})}).listen(3e3);console.error(`${R("Serving at")} ${j("http://localhost:3000")}`),await i.wait,a.close()},It=$e;var Te={...rt,parseMarkdownHeaderTree:B,parseMarkdownTitle:Pt,compileMarkdownToReact:k};function Nt(){console.error(j("Usage:")),console.error("- esbuild-script start: start the devserver."),console.error("- esbuild-script build: generate production build."),console.error("- esbuild-script ssg: generate static site."),console.error("- esbuild-script ssg --no-js: generate static site without JS."),console.error("- esbuild-script help: display command line usages.")}async function _e(t){let e=process.argv[2]||"";switch(e){case"init":return await Ft(),!0;case"start":return await It(t),!0;case"ssg":return lt(t,!0);case"build":return lt(t,!1);case"help":case"--help":return Nt(),!0;default:return console.error(E(`Unknown command: '${e}'`)),Nt(),!1}}async function Ot(t){let e=t?await t():{};try{await _e(e)||(process.exitCode=1)}catch(r){console.error(E(r)),process.exitCode=1}}0&&(module.exports={constants,utils});
})();
//# sourceMappingURL=api.js.map