// @generated
/* eslint-disable */
// prettier-ignore
(() => {
var kt=Object.create,N=Object.defineProperty;var Ft=Object.getOwnPropertyDescriptor;var Nt=Object.getOwnPropertyNames;var Mt=Object.getPrototypeOf,Gt=Object.prototype.hasOwnProperty;var ut=t=>N(t,"__esModule",{value:!0});var Bt=(t,e)=>()=>(e||t((e={exports:{}}).exports,e),e.exports),Z=(t,e)=>{for(var r in e)N(t,r,{get:e[r],enumerable:!0})},Wt=(t,e,r)=>{if(e&&typeof e=="object"||typeof e=="function")for(let o of Nt(e))!Gt.call(t,o)&&o!=="default"&&N(t,o,{get:()=>e[o],enumerable:!(r=Ft(e,o))||r.enumerable});return t},p=t=>Wt(ut(N(t!=null?kt(Mt(t)):{},"default",t&&t.__esModule&&"default"in t?{get:()=>t.default,enumerable:!0}:{value:t,enumerable:!0})),t);var gt=Bt(st=>{function $(t,e){if(typeof t=="string")return t;if(t){let r,o;if(Array.isArray(t)){for(r=0;r<t.length;r++)if(o=$(t[r],e))return o}else for(r in t)if(e.has(r))return $(t[r],e)}}function _(t,e,r){throw new Error(r?`No known conditions for "${e}" entry in "${t}" package`:`Missing "${e}" export in "${t}" package`)}function ft(t,e){return e===t?".":e[0]==="."?e:e.replace(new RegExp("^"+t+"/"),"./")}function Ut(t,e=".",r={}){let{name:o,exports:n}=t;if(n){let{browser:a,require:u,conditions:s=[]}=r,i=ft(o,e);if(i[0]!=="."&&(i="./"+i),typeof n=="string")return i==="."?n:_(o,i);let m=new Set(["default",...s]);m.add(u?"require":"import"),m.add(a?"browser":"node");let d,y,ct=!1;for(d in n){ct=d[0]!==".";break}if(ct)return i==="."?$(n,m)||_(o,i,1):_(o,i);if(y=n[i])return $(y,m)||_(o,i,1);for(d in n){if(y=d[d.length-1],y==="/"&&i.startsWith(d))return(y=$(n[d],m))?y+i.substring(d.length):_(o,i,1);if(y==="*"&&i.startsWith(d.slice(0,-1))&&i.substring(d.length-1).length>0)return(y=$(n[d],m))?y.replace("*",i.substring(d.length-1)):_(o,i,1)}return _(o,i)}}function It(t,e={}){let r=0,o,n=e.browser,a=e.fields||["module","main"];for(n&&!a.includes("browser")&&a.unshift("browser");r<a.length;r++)if(o=t[a[r]]){if(typeof o!="string")if(typeof o=="object"&&a[r]=="browser"){if(typeof n=="string"&&(o=o[n=ft(t.name,n)],o==null))return n}else continue;return typeof o=="string"?"./"+o.replace(/^\.?\//,""):o}}st.legacy=It;st.resolve=Ut});ut(exports);Z(exports,{constants:()=>tt,default:()=>Lt,utils:()=>he});var h=p(require("path")),at=p(require("esbuild"));var wt=p(require("module")),c=p(require("path")),St=p(require("sass"));var tt={};Z(tt,{BUILD_PATH:()=>G,DOCS_PATH:()=>et,GENERATED_PAGES_PATH:()=>x,PAGES_PATH:()=>f,SSR_CSS_PATH:()=>ot,SSR_JS_PATH:()=>L,SSR_LICENSE_PATH:()=>rt,TEMPLATE_PATH:()=>M,TEMP_PATH:()=>S,TEMP_SERVER_ENTRY_PATH:()=>D});var w=p(require("path")),M=(0,w.join)(__dirname,"templates"),S=".temp",D=(0,w.join)(".temp","__server__.jsx"),et=(0,w.join)("docs"),f=(0,w.join)("src","pages"),x=(0,w.join)("src","generated-pages"),L=(0,w.join)("build","__ssr.jsx"),rt=(0,w.join)("build","__ssr.jsx.LEGAL.txt"),ot=(0,w.join)("build","__ssr.css"),G="build";var nt={};Z(nt,{copyDirectoryContent:()=>O,copyFile:()=>k,emptyDirectory:()=>U,ensureDirectory:()=>g,exists:()=>I,isDirectory:()=>W,readDirectory:()=>J,readFile:()=>it,remove:()=>R,writeFile:()=>b});var l=p(require("fs")),E=p(require("path")),v=(t,e)=>r=>r?e(r):t(),B=t=>new Promise((e,r)=>(0,l.readdir)(t,(o,n)=>o?r(o):e(n))),mt=async t=>Promise.all((await B(t)).flatMap(async e=>{let r=(0,E.join)(t,e);return await W(r)?[r,...await mt(r)]:[r]})).then(e=>e.flat()),O=async(t,e)=>{await g(e),await U(e),await Promise.all((await B(t)).map(async r=>{let o=(0,E.join)(t,r),n=(0,E.join)(e,r);await W(o)?await O(o,n):await k(o,n)}))},k=(t,e)=>new Promise((r,o)=>(0,l.copyFile)(t,e,v(r,o))),U=async t=>{let e=await B(t);await Promise.all(e.map(r=>R((0,E.join)(t,r))))},g=t=>new Promise((e,r)=>(0,l.mkdir)(t,{recursive:!0},v(e,r))),I=t=>new Promise(e=>(0,l.access)(t,void 0,r=>e(r==null))),W=t=>new Promise((e,r)=>(0,l.lstat)(t,(o,n)=>o?r(o):e(n.isDirectory()))),J=async(t,e)=>e?await I(t)?(await mt(t)).map(r=>(0,E.relative)(t,r)).sort((r,o)=>r.localeCompare(o)):[]:B(t),it=t=>new Promise((e,r)=>(0,l.readFile)(t,(o,n)=>o?r(o):e(n.toString()))),R=t=>new Promise((e,r)=>{l.readFile!=null?(0,l.rm)(t,{recursive:!0,force:!0},v(e,r)):W(t).then(o=>o?(0,l.rmdir)(t,{recursive:!0},v(e,r)):(0,l.unlink)(t,v(e,r))).catch(o=>r(o))}),b=(t,e)=>new Promise((r,o)=>(0,l.writeFile)(t,e,v(r,o)));var pt=p(require("@mdx-js/mdx")),dt=p(require("remark-slug")),Ot=async t=>`import React from 'react';
import mdx from 'esbuild-scripts/__internal-components__/mdx';
${await(0,pt.default)(t,{remarkPlugins:[dt.default]})}`,Y=Ot;var ht=p(require("fs"));var z=p(require("fs")),P=p(require("path")),q=p(gt()),Jt=[".tsx",".ts",".jsx",".mjs",".cjs",".js",".json"],Yt=/^(\/|\.{1,2}(\/|$))/,zt=/^\.{0,2}\//,qt=(t,e,r)=>{let o=(0,q.resolve)(t,e,{browser:r,require:!r});if(o!=null)return o;let n=(0,q.legacy)(t,r?{browser:r}:{browser:!1,fields:["main","module"]});return typeof n=="string"?e==="."?n:null:n&&(n[e]||n[`${e}.js`]||n[`./${e}`]||n[`./${e}.js`])||null},Vt=(t,e)=>{let r=(0,P.normalize)(t),o=(0,P.normalize)(e);return r===o?".":(r.endsWith(P.sep)||(r=r+P.sep),o.startsWith(r)?o.slice(r.length):null)},Kt=(t,e,r)=>{let o=e.findPackageLocator((0,P.join)(t,"internal.js"));if(o==null)throw new Error;let{packageLocation:n}=e.getPackageInformation(o),a=(0,P.join)(n,"package.json");if(!z.existsSync(a))return null;let u=JSON.parse(z.readFileSync(a,"utf8")),s=Vt(n,t);if(s==null)throw new Error;zt.test(s)||(s=`./${s}`),s=(0,P.normalize)(s);let i=qt(u,s,r);return i!=null?(0,P.join)(n,i):null},Qt=(t,e,r,o)=>{if(Yt.test(t))return e;let n=Kt(e,r,o);return n?(0,P.normalize)(n):e},Xt=(t,e,r,o)=>{let n=r.resolveToUnqualified(t,e);return n==null?null:r.resolveUnqualified(Qt(t,n,r,o),{extensions:Jt})},Pt=Xt;var bt=/()/,Zt=()=>({name:"esbuild-scripts-esbuild-plugin-pnp",setup(t){let{findPnpApi:e}=require("module");if(typeof e=="undefined")return;let r=process.cwd();t.onResolve({filter:bt},async o=>{let n=o.importer?o.importer:`${r}/`,a=e(n);if(!a)return;let u=Pt(o.path,n,a,t.initialOptions.platform!=="node");return u==null?{external:!0}:{namespace:"pnp",path:u}}),t.onLoad!==null&&t.onLoad({filter:bt},async o=>({contents:await ht.promises.readFile(o.path,"utf8"),loader:"default"}))}}),yt=Zt;var te={name:"WebAppResolvePlugin",setup(t){t.onResolve({filter:/^data:/},()=>({external:!0})),t.onResolve({filter:/^esbuild-scripts-internal\/docs\//},e=>({path:(0,c.resolve)((0,c.join)(et,(0,c.relative)((0,c.join)("esbuild-scripts-internal","docs"),e.path)))})),t.onResolve({filter:/^esbuild-scripts-internal\/page\//},async e=>{let r=(0,c.relative)((0,c.join)("esbuild-scripts-internal","page"),e.path),o=[(0,c.join)(f,`${r}.js`),(0,c.join)(f,`${r}.jsx`),(0,c.join)(f,`${r}.ts`),(0,c.join)(f,`${r}.tsx`),(0,c.join)(x,`${r}.js`),(0,c.join)(x,`${r}.jsx`),(0,c.join)(x,`${r}.ts`),(0,c.join)(x,`${r}.tsx`)];for(let n of o)if(await I(n))return{path:(0,c.resolve)(n)};throw new Error(`Cannot found page at ${r}. Candidates considered:
${o.join(`
`)}`)})}},ee={name:"sass",setup(t){t.onResolve({filter:/.\.(scss|sass)$/},async e=>e.path.startsWith(".")?{path:(0,c.resolve)((0,c.dirname)(e.importer),e.path)}:{path:(0,wt.createRequire)(e.importer).resolve(e.path)}),t.onLoad({filter:/.\.(scss|sass)$/},async e=>{let{css:r}=await new Promise((o,n)=>{(0,St.render)({file:e.path},(a,u)=>{a?n(a):o(u)})});return{contents:r.toString(),loader:"css",watchFiles:[e.path]}})}},re={name:"mdx",setup(t){t.onLoad({filter:/\.mdx?$/},async e=>({contents:await Y(await it(e.path)),loader:"jsx"}))}},oe=[te,ee,re,yt()],xt=oe;var ne=({isServer:t=!1,isProd:e=!1})=>({define:{__dirname:'""',__SERVER__:String(t),"process.env.NODE_ENV":e?'"production"':'"development"'},bundle:!0,minify:!1,legalComments:"linked",platform:"browser",target:"es2019",logLevel:"error",plugins:xt}),F=ne;var A=p(require("path"));var Et="// @generated",_t=t=>t.endsWith("index")?t.substring(0,t.length-(t.endsWith("/index")?6:5)):t,ie=(t,e)=>{let r=_t(t),o=e.filter(i=>i!==t),n=o.map(_t),a=o.map((i,m)=>`const Component${m} = lazy(() => import('esbuild-scripts-internal/page/${i}'));`).join(`
`),u=n.map((i,m)=>`<Route exact path="/${i}"><Suspense fallback={null}><Component${m} /></Suspense></Route>`).join(""),s=`<Switch><Route exact path="/${r}"><Page /></Route>${u}</Switch>`;return`${Et}
import React,{Suspense,lazy} from 'react';
import {hydrate,render} from 'react-dom';
import {BrowserRouter,Route,Switch} from 'esbuild-scripts/__internal-components__/react-router';
import Document from 'esbuild-scripts-internal/page/_document';
import Page from 'esbuild-scripts-internal/page/${t}';
${a}
const element = <BrowserRouter><Document>${s}</Document></BrowserRouter>;const rootElement = document.getElementById('root');
if (rootElement.hasChildNodes()) hydrate(element, rootElement); else render(element, rootElement);
`},se=t=>`${Et}
import React from 'react';
import {renderToString} from 'react-dom/server';
import Helmet from 'esbuild-scripts/components/Head';
import {StaticRouter} from 'esbuild-scripts/__internal-components__/react-router';
import Document from 'esbuild-scripts-internal/page/_document';
${t.map((e,r)=>`import Page${r} from 'esbuild-scripts-internal/page/${e}';`).join(`
`)}
const map = { ${t.map((e,r)=>`'${e}': Page${r}`).join(", ")} };
module.exports = (path) => ({
  divHTML: renderToString(
    <StaticRouter location={'/'+path}>
      <Document>{React.createElement(map[path])}</Document>
    </StaticRouter>
  ),
  noJS: map[path].noJS,
  helmet: Helmet.renderStatic(),
});
`,ae=async()=>{await g(f);let[t,e]=await Promise.all([J(f,!0),J(x,!0)]);return[...t,...e].map(o=>{switch((0,A.extname)(o)){case".js":case".jsx":case".ts":case".tsx":break;default:return null}return o.substring(0,o.lastIndexOf("."))}).filter(o=>o!=null&&!o.startsWith("_document"))},V=async()=>{let t=await ae();return await g(S),await U(S),await Promise.all([...t.map(async e=>{let r=(0,A.join)(S,`${e}.jsx`);await g((0,A.dirname)(r)),await b(r,ie(e,t))}),b(D,se(t))]),t};var le=(t,e)=>e?`<script type="module" src="/${t}"></script>`:`<script src="/${t}"></script>`,ce=(t,e)=>`<link rel="${e?"modulepreload":"preload"}" href="/${t}" />`,ue=(t,e,r)=>{let o=[],n=[];t.forEach(s=>{!r&&s.endsWith("js")?o.push(s):s.endsWith("css")&&n.push(s)});let a=n.map(s=>`<link rel="stylesheet" href="/${s}" />`).join("")+o.map(s=>ce(s,e)).join(""),u=o.map(s=>le(s,e)).join("");return{headLinks:a,bodyScriptLinks:u}},Tt=(t,e)=>e==null?`<head>${t}</head>`:`<head>${[e.meta.toString(),e.title.toString(),e.link.toString(),e.script.toString(),t].join("")}</head>`,me=(t,e,r)=>{let{headLinks:o,bodyScriptLinks:n}=ue(e,r,t==null?void 0:t.noJS);if(t==null){let m=Tt(o),d=`<body><div id="root"></div>${n}</body>`;return`<!DOCTYPE html><html>${m}${d}</html>`}let{divHTML:a,helmet:u}=t,s=Tt(o,u),i=`<body><div id="root">${a}</div>${n}</body>`;return`<!DOCTYPE html><html ${u.htmlAttributes.toString()}>${s}${i}</html>`},K=me;var C=t=>process.stderr.isTTY?e=>`[${t}m${e}[0m`:e=>e,H=C(31),j=C(32),vt=C(33),Q=C(34),Ge=C(35),Be=C(36);var Rt=async t=>{let{outputFiles:e}=await(0,at.build)({...F({isProd:!0}),entryPoints:t.map(o=>(0,h.join)(S,`${o}.jsx`)),assetNames:"assets/[name]-[hash]",chunkNames:"chunks/[name]-[hash]",entryNames:"[dir]/[name]-[hash]",minify:!0,format:"esm",splitting:!0,write:!1,outdir:"build"}),r=(0,h.resolve)("build");return await Promise.all(e.map(async o=>{await g((0,h.dirname)(o.path)),await b(o.path,o.contents)})),e.map(({path:o})=>(0,h.relative)(r,o))},pe=async()=>{await(0,at.build)({...F({isServer:!0,isProd:!0}),entryPoints:[D],platform:"node",format:"cjs",outfile:L});try{return require((0,h.resolve)(L))}catch(t){return console.error(H("Unable to perform server side rendering since the server bundle is not correctly generated.")),console.error(H(t)),null}finally{await Promise.all([R(L),R(rt),R(ot)])}},de=async t=>{let e=new Date().getTime();console.error(vt("[i] Bundling..."));let r=await V();await O("public","build");let o,n;if(t){if([o,n]=await Promise.all([Rt(r),pe()]),n==null)return!1}else o=await Rt(r),n=null;let a=r.map(s=>{let i=o.filter(d=>d.startsWith("chunk")||d.startsWith(s)),m=K(n==null?void 0:n(s),i,!0);return{entryPoint:s,html:m}});await Promise.all(a.map(async({entryPoint:s,html:i})=>{let m;s.endsWith("index")?m=(0,h.join)(G,`${s}.html`):m=(0,h.join)(G,s,"index.html"),await g((0,h.dirname)(m)),await b(m,i)}));let u=new Date().getTime()-e;return console.error(`\u26A1 ${j(`Build success in ${u}ms.`)}`),!0},lt=de;var T=p(require("path"));var fe=async()=>{await g(f),await Promise.all([k((0,T.join)(M,"_document.tsx"),(0,T.join)(f,"_document.tsx")),k((0,T.join)(M,"index.tsx"),(0,T.join)(f,"index.tsx")),b((0,T.join)(f,"index.css"),""),b((0,T.join)("src","types.d.ts"),`/// <reference types="esbuild-scripts" />
`),g("public")]),console.error(j("esbuild-scripts app initialized."))},$t=fe;var X=p(require("http")),At=p(require("path")),Ct=p(require("esbuild"));var ge=(t,e)=>{if(e==null||!e.startsWith("/"))return;let r=e.substring(1);return t.find(o=>o.endsWith("index")?[o,o.substring(0,o.length-5)].includes(r):o===r)},Ht=t=>K(void 0,[`${t}.js`,`${t}.css`],!1),Pe=async()=>{let t=await V(),e=await(0,Ct.serve)({servedir:"public",port:19815},{...F({}),entryPoints:t.map(o=>(0,At.join)(S,`${o}.jsx`)),sourcemap:"inline",outdir:"public"}),r=(0,X.createServer)((o,n)=>{let a=ge(t,o.url);if(a!=null){n.writeHead(200,{"Content-Type":"text/html"}),n.end(Ht(a));return}let u={hostname:e.host,port:e.port,path:o.url,method:o.method,headers:o.headers},s=(0,X.request)(u,i=>{if(i.statusCode===404){n.writeHead(200,{"Content-Type":"text/html"}),n.end(Ht("index"));return}n.writeHead(i.statusCode||200,i.headers),i.pipe(n,{end:!0})});o.pipe(s,{end:!0})}).listen(3e3);console.error(`${j("Serving at")} ${Q("http://localhost:3000")}`),await e.wait,r.close()},jt=Pe;var he={...nt,compileMarkdownToReact:Y};function Dt(){console.error(Q("Usage:")),console.error("- esbuild-script start: start the devserver."),console.error("- esbuild-script build: generate production build."),console.error("- esbuild-script ssg: generate static site."),console.error("- esbuild-script ssg --no-js: generate static site without JS."),console.error("- esbuild-script help: display command line usages.")}async function be(){let t=process.argv[2]||"";switch(t){case"init":return await $t(),!0;case"start":return await jt(),!0;case"ssg":return lt(!0);case"build":return lt(!1);case"help":case"--help":return Dt(),!0;default:return console.error(H(`Unknown command: '${t}'`)),Dt(),!1}}async function Lt(t){t&&await t();try{await be()||(process.exitCode=1)}catch(e){console.error(H(e)),process.exitCode=1}}0&&(module.exports={constants,utils});
})();
