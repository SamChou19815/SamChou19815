#!/usr/bin/env node --unhandled-rejections=strict
/* eslint-disable */
// prettier-ignore
(()=>{var At=Object.create,I=Object.defineProperty,Ct=Object.getPrototypeOf,Ht=Object.prototype.hasOwnProperty,jt=Object.getOwnPropertyNames,Dt=Object.getOwnPropertyDescriptor;var Lt=t=>I(t,"__esModule",{value:!0});var Ft=(t,e)=>()=>(e||t((e={exports:{}}).exports,e),e.exports);var kt=(t,e,n)=>{if(e&&typeof e=="object"||typeof e=="function")for(let r of jt(e))!Ht.call(t,r)&&r!=="default"&&I(t,r,{get:()=>e[r],enumerable:!(n=Dt(e,r))||n.enumerable});return t},p=t=>kt(Lt(I(t!=null?At(Ct(t)):{},"default",t&&t.__esModule&&"default"in t?{get:()=>t.default,enumerable:!0}:{value:t,enumerable:!0})),t);var st=Ft(z=>{function v(t,e){if(typeof t=="string")return t;if(t){let n,r;if(Array.isArray(t)){for(n=0;n<t.length;n++)if(r=v(t[n],e))return r}else for(n in t)if(e.has(n))return v(t[n],e)}}function E(t,e,n){throw new Error(n?`No known conditions for "${e}" entry in "${t}" package`:`Missing "${e}" export in "${t}" package`)}function it(t,e){return e===t?".":e[0]==="."?e:e.replace(new RegExp("^"+t+"/"),"./")}function Nt(t,e=".",n={}){let{name:r,exports:o}=t;if(o){let{browser:a,require:u,conditions:s=[]}=n,i=it(r,e);if(i[0]!=="."&&(i="./"+i),typeof o=="string")return i==="."?o:E(r,i);let m=new Set(["default",...s]);m.add(u?"require":"import"),m.add(a?"browser":"node");let d,b,et=!1;for(d in o){et=d[0]!==".";break}if(et)return i==="."?v(o,m)||E(r,i,1):E(r,i);if(b=o[i])return v(b,m)||E(r,i,1);for(d in o){if(b=d[d.length-1],b==="/"&&i.startsWith(d))return(b=v(o[d],m))?b+i.substring(d.length):E(r,i,1);if(b==="*"&&i.startsWith(d.slice(0,-1))&&i.substring(d.length-1).length>0)return(b=v(o[d],m))?b.replace("*",i.substring(d.length-1)):E(r,i,1)}return E(r,i)}}function Gt(t,e={}){let n=0,r,o=e.browser,a=e.fields||["module","main"];for(o&&!a.includes("browser")&&a.unshift("browser");n<a.length;n++)if(r=t[a[n]]){if(typeof r!="string")if(typeof r=="object"&&a[n]=="browser"){if(typeof o=="string"&&(r=r[o=it(t.name,o)],r==null))return o}else continue;return typeof r=="string"?"./"+r.replace(/^\.?\//,""):r}}z.legacy=Gt;z.resolve=Nt});var P=p(require("path")),Z=p(require("esbuild"));var dt=p(require("module")),c=p(require("path")),ft=p(require("@mdx-js/mdx")),gt=p(require("remark-slug")),Pt=p(require("sass"));var y=p(require("path")),J=(0,y.join)(__dirname,"templates"),w=".temp",L=(0,y.join)(".temp","__server__.jsx"),rt=(0,y.join)("docs"),f=(0,y.join)("src","pages"),x=(0,y.join)("src","generated-pages"),F=(0,y.join)("build","__ssr.jsx"),nt=(0,y.join)("build","__ssr.jsx.LEGAL.txt"),ot=(0,y.join)("build","__ssr.css"),Y="build";var lt=p(require("fs"));var k=p(require("fs")),g=p(require("path")),N=p(st()),Mt=[".tsx",".ts",".jsx",".mjs",".cjs",".js",".json"],Bt=/^(\/|\.{1,2}(\/|$))/,Wt=/^\.{0,2}\//,Ot=(t,e,n)=>{let r=(0,N.resolve)(t,e,{browser:n,require:!n});if(r!=null)return r;let o=(0,N.legacy)(t,n?{browser:n}:{browser:!1,fields:["main","module"]});return typeof o=="string"?e==="."?o:null:o&&(o[e]||o[`${e}.js`]||o[`./${e}`]||o[`./${e}.js`])||null},Ut=(t,e)=>{let n=(0,g.normalize)(t),r=(0,g.normalize)(e);return n===r?".":(n.endsWith(g.sep)||(n=n+g.sep),r.startsWith(n)?r.slice(n.length):null)},It=(t,e,n)=>{let r=e.findPackageLocator((0,g.join)(t,"internal.js"));if(r==null)throw new Error;let{packageLocation:o}=e.getPackageInformation(r),a=(0,g.join)(o,"package.json");if(!k.existsSync(a))return null;let u=JSON.parse(k.readFileSync(a,"utf8")),s=Ut(o,t);if(s==null)throw new Error;Wt.test(s)||(s=`./${s}`),s=(0,g.normalize)(s);let i=Ot(u,s,n);return i!=null?(0,g.join)(o,i):null},Jt=(t,e,n,r)=>{if(Bt.test(t))return e;let o=It(e,n,r);return o?(0,g.normalize)(o):e},Yt=(t,e,n,r)=>{let o=n.resolveToUnqualified(t,e);return o==null?null:n.resolveUnqualified(Jt(t,o,n,r),{extensions:Mt})},at=Yt;var ct=/()/,zt=()=>({name:"esbuild-scripts-esbuild-plugin-pnp",setup(t){let{findPnpApi:e}=require("module");if(typeof e=="undefined")return;let n=process.cwd();t.onResolve({filter:ct},async r=>{let o=r.importer?r.importer:`${n}/`,a=e(o);if(!a)return;let u=at(r.path,o,a,t.initialOptions.platform!=="node");return u==null?{external:!0}:{namespace:"pnp",path:u}}),t.onLoad!==null&&t.onLoad({filter:ct},async r=>({contents:await lt.promises.readFile(r.path,"utf8"),loader:"default"}))}}),ut=zt;var l=p(require("fs")),_=p(require("path")),$=(t,e)=>n=>n?e(n):t(),G=t=>new Promise((e,n)=>(0,l.readdir)(t,(r,o)=>r?n(r):e(o))),mt=async t=>Promise.all((await G(t)).flatMap(async e=>{let n=(0,_.join)(t,e);return await q(n)?[n,...await mt(n)]:[n]})).then(e=>e.flat()),V=async(t,e)=>{await h(e),await K(e),await Promise.all((await G(t)).map(async n=>{let r=(0,_.join)(t,n),o=(0,_.join)(e,n);await q(r)?await V(r,o):await M(r,o)}))},M=(t,e)=>new Promise((n,r)=>(0,l.copyFile)(t,e,$(n,r))),K=async t=>{let e=await G(t);await Promise.all(e.map(n=>j((0,_.join)(t,n))))},h=t=>new Promise((e,n)=>(0,l.mkdir)(t,{recursive:!0},$(e,n))),Q=t=>new Promise(e=>(0,l.access)(t,void 0,n=>e(n==null))),q=t=>new Promise((e,n)=>(0,l.lstat)(t,(r,o)=>r?n(r):e(o.isDirectory()))),X=async(t,e)=>e?await Q(t)?(await mt(t)).map(n=>(0,_.relative)(t,n)).sort((n,r)=>n.localeCompare(r)):[]:G(t),pt=t=>new Promise((e,n)=>(0,l.readFile)(t,(r,o)=>r?n(r):e(o.toString()))),j=t=>new Promise((e,n)=>{l.readFile!=null?(0,l.rm)(t,{recursive:!0,force:!0},$(e,n)):q(t).then(r=>r?(0,l.rmdir)(t,{recursive:!0},$(e,n)):(0,l.unlink)(t,$(e,n))).catch(r=>n(r))}),S=(t,e)=>new Promise((n,r)=>(0,l.writeFile)(t,e,$(n,r)));var qt={name:"WebAppResolvePlugin",setup(t){t.onResolve({filter:/^data:/},()=>({external:!0})),t.onResolve({filter:/^esbuild-scripts-internal\/docs\//},e=>({path:(0,c.resolve)((0,c.join)(rt,(0,c.relative)((0,c.join)("esbuild-scripts-internal","docs"),e.path)))})),t.onResolve({filter:/^esbuild-scripts-internal\/page\//},async e=>{let n=(0,c.relative)((0,c.join)("esbuild-scripts-internal","page"),e.path),r=[(0,c.join)(f,`${n}.js`),(0,c.join)(f,`${n}.jsx`),(0,c.join)(f,`${n}.ts`),(0,c.join)(f,`${n}.tsx`),(0,c.join)(x,`${n}.js`),(0,c.join)(x,`${n}.jsx`),(0,c.join)(x,`${n}.ts`),(0,c.join)(x,`${n}.tsx`)];for(let o of r)if(await Q(o))return{path:(0,c.resolve)(o)};throw new Error(`Cannot found page at ${n}. Candidates considered:
${r.join(`
`)}`)})}},Vt={name:"sass",setup(t){t.onResolve({filter:/.\.(scss|sass)$/},async e=>e.path.startsWith(".")?{path:(0,c.resolve)((0,c.dirname)(e.importer),e.path)}:{path:(0,dt.createRequire)(e.importer).resolve(e.path)}),t.onLoad({filter:/.\.(scss|sass)$/},async e=>{let{css:n}=await new Promise((r,o)=>{(0,Pt.render)({file:e.path},(a,u)=>{a?o(a):r(u)})});return{contents:n.toString(),loader:"css",watchFiles:[e.path]}})}},Kt={name:"mdx",setup(t){t.onLoad({filter:/\.mdx?$/},async e=>{let n=await pt(e.path);return{contents:`import React from'react';
import mdx from 'esbuild-scripts/__internal-components__/mdx';
${await(0,ft.default)(n,{remarkPlugins:[gt.default]})}`,loader:"jsx"}})}},Qt=[qt,Vt,Kt,ut()],ht=Qt;var Xt=({isServer:t=!1,isProd:e=!1})=>({define:{__dirname:'""',__SERVER__:String(t),"process.env.NODE_ENV":e?'"production"':'"development"'},bundle:!0,minify:!1,legalComments:"linked",platform:"browser",target:"es2019",logLevel:"error",plugins:ht}),D=Xt;var R=p(require("path"));var bt="// @generated",yt=t=>t.endsWith("index")?t.substring(0,t.length-(t.endsWith("/index")?6:5)):t,Zt=(t,e)=>{let n=yt(t),r=e.filter(i=>i!==t),o=r.map(yt),a=r.map((i,m)=>`const Component${m} = lazy(() => import('esbuild-scripts-internal/page/${i}'));`).join(`
`),u=o.map((i,m)=>`<Route exact path="/${i}"><Suspense fallback={null}><Component${m} /></Suspense></Route>`).join(""),s=`<Switch><Route exact path="/${n}"><Page /></Route>${u}</Switch>`;return`${bt}
import React,{Suspense,lazy} from 'react';
import {hydrate,render} from 'react-dom';
import {BrowserRouter,Route,Switch} from 'esbuild-scripts/__internal-components__/react-router';
import Document from 'esbuild-scripts-internal/page/_document';
import Page from 'esbuild-scripts-internal/page/${t}';
${a}
const element = <BrowserRouter><Document>${s}</Document></BrowserRouter>;const rootElement = document.getElementById('root');
if (rootElement.hasChildNodes()) hydrate(element, rootElement); else render(element, rootElement);
`},te=t=>`${bt}
import React from 'react';
import {renderToString} from 'react-dom/server';
import Helmet from 'esbuild-scripts/components/Head';
import {StaticRouter} from 'esbuild-scripts/__internal-components__/react-router';
import Document from 'esbuild-scripts-internal/page/_document';
${t.map((e,n)=>`import Page${n} from 'esbuild-scripts-internal/page/${e}';`).join(`
`)}
const map = { ${t.map((e,n)=>`'${e}': Page${n}`).join(", ")} };
module.exports = (path) => ({
  divHTML: renderToString(
    <StaticRouter location={'/'+path}>
      <Document>{React.createElement(map[path])}</Document>
    </StaticRouter>
  ),
  noJS: map[path].noJS,
  helmet: Helmet.renderStatic(),
});
`,ee=async()=>{await h(f);let[t,e]=await Promise.all([X(f,!0),X(x,!0)]);return[...t,...e].map(r=>{switch((0,R.extname)(r)){case".js":case".jsx":case".ts":case".tsx":break;default:return null}return r.substring(0,r.lastIndexOf("."))}).filter(r=>r!=null&&!r.startsWith("_document"))},B=async()=>{let t=await ee();return await h(w),await K(w),await Promise.all([...t.map(async e=>{let n=(0,R.join)(w,`${e}.jsx`);await h((0,R.dirname)(n)),await S(n,Zt(e,t))}),S(L,te(t))]),t};var re=(t,e)=>e?`<script type="module" src="/${t}"></script>`:`<script src="/${t}"></script>`,ne=(t,e)=>`<link rel="${e?"modulepreload":"preload"}" href="/${t}" />`,oe=(t,e,n)=>{let r=[],o=[];t.forEach(s=>{!n&&s.endsWith("js")?r.push(s):s.endsWith("css")&&o.push(s)});let a=o.map(s=>`<link rel="stylesheet" href="/${s}" />`).join("")+r.map(s=>ne(s,e)).join(""),u=r.map(s=>re(s,e)).join("");return{headLinks:a,bodyScriptLinks:u}},St=(t,e)=>e==null?`<head>${t}</head>`:`<head>${[e.meta.toString(),e.title.toString(),e.link.toString(),e.script.toString(),t].join("")}</head>`,ie=(t,e,n)=>{let{headLinks:r,bodyScriptLinks:o}=oe(e,n,t==null?void 0:t.noJS);if(t==null){let m=St(r),d=`<body><div id="root"></div>${o}</body>`;return`<!DOCTYPE html><html>${m}${d}</html>`}let{divHTML:a,helmet:u}=t,s=St(r,u),i=`<body><div id="root">${a}</div>${o}</body>`;return`<!DOCTYPE html><html ${u.htmlAttributes.toString()}>${s}${i}</html>`},W=ie;var A=t=>process.stderr.isTTY?e=>`[${t}m${e}[0m`:e=>e,C=A(31),H=A(32),wt=A(33),O=A(34),De=A(35),Le=A(36);var xt=async t=>{let{outputFiles:e}=await(0,Z.build)({...D({isProd:!0}),entryPoints:t.map(r=>(0,P.join)(w,`${r}.jsx`)),assetNames:"assets/[name]-[hash]",chunkNames:"chunks/[name]-[hash]",entryNames:"[dir]/[name]-[hash]",minify:!0,format:"esm",splitting:!0,write:!1,outdir:"build"}),n=(0,P.resolve)("build");return await Promise.all(e.map(async r=>{await h((0,P.dirname)(r.path)),await S(r.path,r.contents)})),e.map(({path:r})=>(0,P.relative)(n,r))},se=async()=>{await(0,Z.build)({...D({isServer:!0,isProd:!0}),entryPoints:[L],platform:"node",format:"cjs",outfile:F});try{return require((0,P.resolve)(F))}catch(t){return console.error(C("Unable to perform server side rendering since the server bundle is not correctly generated.")),console.error(C(t)),null}finally{await Promise.all([j(F),j(nt),j(ot)])}},ae=async t=>{let e=new Date().getTime();console.error(wt("[i] Bundling..."));let n=await B();await V("public","build");let r,o;if(t){if([r,o]=await Promise.all([xt(n),se()]),o==null)return!1}else r=await xt(n),o=null;let a=n.map(s=>{let i=r.filter(d=>d.startsWith("chunk")||d.startsWith(s)),m=W(o==null?void 0:o(s),i,!0);return{entryPoint:s,html:m}});await Promise.all(a.map(async({entryPoint:s,html:i})=>{let m;s.endsWith("index")?m=(0,P.join)(Y,`${s}.html`):m=(0,P.join)(Y,s,"index.html"),await h((0,P.dirname)(m)),await S(m,i)}));let u=new Date().getTime()-e;return console.error(`\u26A1 ${H(`Build success in ${u}ms.`)}`),!0},tt=ae;var T=p(require("path"));var le=async()=>{await h(f),await Promise.all([M((0,T.join)(J,"_document.tsx"),(0,T.join)(f,"_document.tsx")),M((0,T.join)(J,"index.tsx"),(0,T.join)(f,"index.tsx")),S((0,T.join)(f,"index.css"),""),S((0,T.join)("src","types.d.ts"),`/// <reference types="esbuild-scripts" />
`),h("public")]),console.error(H("esbuild-scripts app initialized."))},Et=le;var U=p(require("http")),_t=p(require("path")),Tt=p(require("esbuild"));var ce=(t,e)=>{if(e==null||!e.startsWith("/"))return;let n=e.substring(1);return t.find(r=>r.endsWith("index")?[r,r.substring(0,r.length-5)].includes(n):r===n)},vt=t=>W(void 0,[`${t}.js`,`${t}.css`],!1),ue=async()=>{let t=await B(),e=await(0,Tt.serve)({servedir:"public",port:19815},{...D({}),entryPoints:t.map(r=>(0,_t.join)(w,`${r}.jsx`)),sourcemap:"inline",outdir:"public"}),n=(0,U.createServer)((r,o)=>{let a=ce(t,r.url);if(a!=null){o.writeHead(200,{"Content-Type":"text/html"}),o.end(vt(a));return}let u={hostname:e.host,port:e.port,path:r.url,method:r.method,headers:r.headers},s=(0,U.request)(u,i=>{if(i.statusCode===404){o.writeHead(200,{"Content-Type":"text/html"}),o.end(vt("index"));return}o.writeHead(i.statusCode||200,i.headers),i.pipe(o,{end:!0})});r.pipe(s,{end:!0})}).listen(3e3);console.error(`${H("Serving at")} ${O("http://localhost:3000")}`),await e.wait,n.close()},$t=ue;function Rt(){console.error(O("Usage:")),console.error("- esbuild-script start: start the devserver."),console.error("- esbuild-script build: generate production build."),console.error("- esbuild-script ssg: generate static site."),console.error("- esbuild-script ssg --no-js: generate static site without JS."),console.error("- esbuild-script help: display command line usages.")}async function me(){let t=process.argv[2]||"";switch(t){case"init":return await Et(),!0;case"start":return await $t(),!0;case"ssg":return tt(!0);case"build":return tt(!1);case"help":case"--help":return Rt(),!0;default:return console.error(C(`Unknown command: '${t}'`)),Rt(),!1}}async function pe(){try{await me()||(process.exitCode=1)}catch(t){console.error(C(t)),process.exitCode=1}}pe();})();
