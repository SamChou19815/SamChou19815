#!/usr/bin/env node
/* eslint-disable */
// prettier-ignore
(()=>{var M=Object.create,C=Object.defineProperty,k=Object.getPrototypeOf,G=Object.prototype.hasOwnProperty,O=Object.getOwnPropertyNames,V=Object.getOwnPropertyDescriptor;var q=e=>C(e,"__esModule",{value:!0});var z=(e,t,r)=>{if(t&&typeof t=="object"||typeof t=="function")for(let o of O(t))!G.call(e,o)&&o!=="default"&&C(e,o,{get:()=>t[o],enumerable:!(r=V(t,o))||r.enumerable});return e},s=e=>z(q(C(e!=null?M(k(e)):{},"default",e&&e.__esModule&&"default"in e?{get:()=>e.default,enumerable:!0}:{value:e,enumerable:!0})),e);var c=s(require("path")),R=s(require("esbuild")),n=s(require("fs-extra"));var a=s(require("path")),S=(0,a.join)(__dirname,"entries","client.ts"),A=(0,a.join)(__dirname,"entries","server.ts"),g=(0,a.join)("build","__ssr.js"),B=(0,a.join)("build","__ssr.css"),x=(0,a.join)("build","index.html"),le=(0,a.join)("build","app.js"),ae=(0,a.join)("build","app.css");var y=s(require("path")),F=s(require("@yarnpkg/esbuild-plugin-pnp")),H=s(require("esbuild-plugin-sass")),K={name:"WebAppResolvePlugin",setup(e){e.onResolve({filter:/data:/},()=>({external:!0})),e.onResolve({filter:/USER_DEFINED_APP_ENTRY_POINT/},()=>({path:(0,y.resolve)((0,y.join)("src","App.tsx"))}))}},Q=[K,(0,H.default)(),(0,F.pnpPlugin)()],D=Q;var X=({isServer:e=!1,isProd:t=!1})=>({define:{__SERVER__:String(e),"process.env.NODE_ENV":t?'"production"':'"development"'},bundle:!0,minify:!1,target:"es2019",logLevel:"info",plugins:D}),f=X;var $=s(require("html-minifier")),h=s(require("node-html-parser")),Z=(e,t)=>(0,h.parse)(t?`<script type="module" src="${e}"></script>`:`<script src="${e}"></script>`),ee=(e,t)=>(0,h.parse)(`<link rel="${t?"modulepreload":"preload"}" href="${e}" />`),te=(e,t,r,{esModule:o,noJS:i=!1})=>{let l=(0,h.parse)(e),b=l.querySelector("head"),u=l.querySelector("body");return t!=null&&(l.querySelector("#root").innerHTML=t),r.forEach(T=>{let v=`/${T}`;T.endsWith("js")&&!i?(u.appendChild(Z(v,o)),b.appendChild(ee(v,o))):T.endsWith("css")&&b.appendChild((0,h.parse)(`<link rel="stylesheet" href="${v}" />`))}),(0,$.minify)(l.toString(),{minifyCSS:!1,minifyJS:!1,collapseWhitespace:!0,collapseInlineTagWhitespace:!0})},E=te;var p=e=>process.stderr.isTTY?t=>`[${e}m${t}[0m`:t=>t,d=p(31),_=p(32),j=p(33),m=p(34),fe=p(35),he=p(36);async function U(){let{outputFiles:e}=await(0,R.build)({...f({isProd:!0}),entryPoints:[S],assetNames:"assets/[name]-[hash]",chunkNames:"chunks/[name]-[hash]",entryNames:"[dir]/[name]-[hash]",minify:!0,format:"esm",splitting:!0,write:!1,outdir:"build"}),t=(0,c.resolve)("build");return await Promise.all(e.map(async r=>{await(0,n.ensureDir)((0,c.dirname)(r.path)),await(0,n.writeFile)(r.path,r.contents,{})})),e.map(({path:r})=>(0,c.relative)(t,r))}async function oe(){await(0,R.build)({...f({isServer:!0,isProd:!0}),entryPoints:[A],platform:"node",format:"cjs",logLevel:"error",outfile:g});try{return require((0,c.resolve)(g))}catch(e){return console.error(d("Unable to perform server side rendering since the server bundle is not correctly generated.")),console.error(d(e)),null}finally{await Promise.all([(0,n.remove)(g),(0,n.remove)(B)])}}async function I(e,t,r){let o=await(0,n.readFile)(x),i=E(o.toString(),e,t,{esModule:!0,noJS:r});await(0,n.writeFile)(x,i)}async function P({staticSiteGeneration:e,noJS:t}){let r=new Date().getTime();if(console.error(j("[i] Bundling...")),await(0,n.ensureDir)("build"),await(0,n.emptyDir)("build"),await(0,n.copy)("public","build"),e){let[i,l]=await Promise.all([U(),oe()]);if(l==null)return!1;await I(l,i,t)}else{let i=await U();await I(null,i,t)}let o=new Date().getTime()-r;return console.error(`\u26A1 ${_(`Build success in ${o}ms.`)}`),!0}var w=s(require("http")),L=s(require("path")),W=s(require("esbuild")),Y=s(require("fs-extra"));async function N(){let e=E((await(0,Y.readFile)((0,L.join)("public","index.html"))).toString(),"",["app.js","app.css"],{esModule:!1}),t=await(0,W.serve)({servedir:"public",port:19815},{...f({}),entryPoints:[S],sourcemap:"inline",outfile:(0,L.join)("public","app.js")});console.error(m(`[i] ESBuild Server started on http://${t.host}:${t.port}.`));let r=(0,w.createServer)((o,i)=>{if(o.url==="/"){i.writeHead(200,{"Content-Type":"text/html"}),i.end(e);return}let l={hostname:t.host,port:t.port,path:o.url,method:o.method,headers:o.headers},b=(0,w.request)(l,u=>{if(u.statusCode===404){i.writeHead(200,{"Content-Type":"text/html"}),i.end(e);return}i.writeHead(u.statusCode||200,u.headers),u.pipe(i,{end:!0})});o.pipe(b,{end:!0})}).listen(3e3);console.error(m("[i] Proxy Server started.")),console.error(`${_("Serving at")} ${m("http://localhost:3000")}`),await t.wait,r.close()}function J(){console.error(m("Usage:")),console.error("- esbuild-script start: start the devserver."),console.error("- esbuild-script build: generate production build."),console.error("- esbuild-script ssg: generate static site."),console.error("- esbuild-script ssg --no-js: generate static site without JS."),console.error("- esbuild-script help: display command line usages.")}async function re(){let e=process.argv[2]||"";switch(e){case"start":return await N(),!0;case"ssg":return P({staticSiteGeneration:!0,noJS:process.argv.includes("--no-js")});case"build":return P({staticSiteGeneration:!1,noJS:!1});case"help":case"--help":return J(),!0;default:return console.error(d(`Unknown command: '${e}'`)),J(),!1}}async function ne(){try{await re()||(process.exitCode=1)}catch(e){console.error(d(e)),process.exitCode=1}}ne();})();
