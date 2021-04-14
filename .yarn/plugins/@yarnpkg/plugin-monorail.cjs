/* eslint-disable */
module.exports = {
name: "@yarnpkg/plugin-monorail",
factory: function (require) {
var plugin;plugin=(()=>{"use strict";var n={97:(n,e,r)=>{r.r(e),r.d(e,{default:()=>b});const o=require("fs"),t=require("clipanion"),a=require("path"),i='\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v2\n        with:\n          fetch-depth: "2"\n      - uses: actions/setup-node@v2\n      - uses: actions/cache@v2\n        with:\n          path: ".yarn/cache\\n.pnp.js"\n          key: "yarn-berry-${{ hashFiles(\'**/yarn.lock\') }}"\n          restore-keys: "yarn-berry-"\n      - name: Yarn Install\n        run: yarn install --immutable',s=n=>[[".github/workflows/generated-general.yml",`# @generated\n\nname: General\non:\n  push:\n    branches:\n      - main\n  pull_request:\n\njobs:\n  lint:${i}\n      - name: Format Check\n        run: yarn format:check\n      - name: Lint\n        run: yarn lint\n  build:${i}\n      - name: Compile\n        run: yarn c\n  validate:${i}\n      - name: Check changed\n        run: if [[ \`git status --porcelain\` ]]; then exit 1; fi\n  test:${i}\n      - name: Test\n        run: yarn test\n`],...n.topologicallyOrdered.filter(e=>((n,e,r)=>{var t,i;const s=n.information[e];if(null==s)throw new Error;return null!=(null===(i=null===(t=JSON.parse((0,o.readFileSync)((0,a.join)(s.workspaceLocation,"package.json")).toString()))||void 0===t?void 0:t.scripts)||void 0===i?void 0:i[r])})(n,e,"deploy")).map(e=>{var r,o;return[`.github/workflows/generated-${"cd-"+e}.yml`,`# @generated\n\nname: CD ${e}\non:\n  push:\n    paths:${(null!==(o=null===(r=n.information[e])||void 0===r?void 0:r.dependencyChain)&&void 0!==o?o:[]).map(e=>{var r;return`\n      - '${null===(r=n.information[e])||void 0===r?void 0:r.workspaceLocation}/**'`}).join("")}\n      - 'configuration/**'\n      - '.github/workflows/generated-*-${e}.yml'\n    branches:\n      - main\nenv:\n  NETLIFY_AUTH_TOKEN: \${{ secrets.NETLIFY_AUTH_TOKEN }}\n\njobs:\n  deploy:${i}\n      - name: Build\n        run: yarn workspace ${e} build\n      - name: Deploy\n        run: yarn workspace ${e} deploy\n`]})],l=(0,a.join)(".github","workflows"),c=async n=>{(0,o.existsSync)(l)&&(0,o.readdirSync)(l).forEach(n=>{n.startsWith("generated-")&&(0,o.unlinkSync)((0,a.join)(l,n))}),(0,o.mkdirSync)(l,{recursive:!0}),s(n).forEach(([n,e])=>{(0,o.writeFileSync)(n,e)})};var d=r(129);const u=n=>process.stderr.isTTY?e=>`[${n}m${e}[0m`:n=>n,p=u(31),m=u(32),y=u(33),h=u(34),f=u(35),v=(u(36),["⠋","⠙","⠹","⠸","⠼","⠴","⠦","⠧","⠇","⠏"]),w=n=>{let e=0;const r=(new Date).getTime();return setInterval(()=>{const o=(((new Date).getTime()-r)/1e3).toFixed(1)+"s",t=n(o),a=v[e%10];process.stderr.write(y(`${t} ${a}\r`)),e+=1},process.stderr.isTTY?40:1e3)},g=(n,e)=>{var r,o;const t=r=>{var o,t;return(0,a.dirname)(r)!==(0,a.join)(null!==(t=null===(o=n.information[e])||void 0===o?void 0:o.workspaceLocation)&&void 0!==t?t:".","bin")};return(null!==(o=null===(r=n.information[e])||void 0===r?void 0:r.dependencyChain)&&void 0!==o?o:[]).some(e=>{var r,o;return(n=>{const e=(e,r)=>{const o=(0,d.spawnSync)("git",["diff",e,...r?[r]:[],"--name-only","--",n]).stdout.toString().trim();return""===o?[]:o.split("\n")};return process.env.CI?e("HEAD^","HEAD"):e("origin/main")})(null!==(o=null===(r=n.information[e])||void 0===r?void 0:r.workspaceLocation)&&void 0!==o?o:".").some(t)})},k=async()=>{var n;const e=(n=>n.topologicallyOrdered.map(e=>[e,g(n,e)]).filter(([,n])=>n).map(([n])=>n))((n="workspaces.json",JSON.parse((0,o.readFileSync)(n).toString())));e.forEach(n=>{console.error(h(`[i] \`${n}\` needs to be recompiled.`))});const r=w(n=>`[?] Compiling (${n})`),t=Promise.all(e.map(n=>{const e=(0,d.spawn)("yarn",["workspace",n,"compile"],{shell:!0,stdio:["ignore","pipe","ignore"]});let r="";return e.stdout.on("data",n=>{r+=n.toString()}),new Promise(o=>{e.on("exit",e=>o([n,0===e,r]))})})),a=await t;clearInterval(r);const i=a.map(n=>n[2]).join(""),s=a.filter(n=>!n[1]).map(n=>n[0]);return 0===s.length?(console.error(m("[✓] All workspaces have been successfully compiled!")),!0):(console.error(f("[!] Compilation finished with some errors.")),console.error(i.trim()),s.forEach(n=>{console.error(p(`[x] \`${n}\` failed to exit with 0.`))}),!1)};class S extends t.Command{async execute(){return await k()?0:1}}S.addPath("c");const b={hooks:{afterAllInstalled:()=>{const n=r(904).Z;(0,o.writeFileSync)("workspaces.json",JSON.stringify(n,void 0,2)+"\n"),c(n)}},commands:[S]}},904:(n,e,r)=>{r.d(e,{Z:()=>i});var o=r(129);const t=(()=>{const n=new Map,e=`[${(0,o.spawnSync)("yarn",["workspaces","list","-v","--json"]).stdout.toString().trim().split("\n").join(",")}]`,r=JSON.parse(e),t={};return r.forEach(({name:n,location:e})=>{null!=n&&(t[e]=n)}),r.forEach(({name:e,location:r,workspaceDependencies:o})=>{null!=e&&n.set(e,{workspaceLocation:r,dependencies:o.map(n=>{const e=t[n];if(null==e)throw new Error(`Bad dependency of ${e}: ${n}`);return e})})}),n})(),a=n=>{const e=[],r=[],o=new Set,a=new Set,i=s=>{var l;if(a.has(s)){if(!o.has(s))return;r.push(s);const n=r.indexOf(s),e=r.slice(n,r.length).join(" -> ");throw new Error("Cyclic dependency detected: "+e)}const c=null===(l=t.get(s))||void 0===l?void 0:l.dependencies;if(null==c)throw new Error(`Workspace ${n} is not found!`);a.add(s),r.push(s),o.add(s),c.forEach(i),o.delete(s),r.pop(),e.push(s)};return i(n),e},i={__type__:"@generated",information:Object.fromEntries(Array.from(t.entries()).map(([n,{dependencies:e,...r}])=>[n,{...r,dependencyChain:a(n)}]).sort(([n],[e])=>n.localeCompare(e))),topologicallyOrdered:(()=>{const n=[],e=new Set;return Array.from(t.keys()).forEach(r=>{a(r).forEach(r=>{e.has(r)||(n.push(r),e.add(r))})}),n})()}},129:n=>{n.exports=require("child_process")}},e={};function r(o){if(e[o])return e[o].exports;var t=e[o]={exports:{}};return n[o](t,t.exports,r),t.exports}return r.n=n=>{var e=n&&n.__esModule?()=>n.default:()=>n;return r.d(e,{a:e}),e},r.d=(n,e)=>{for(var o in e)r.o(e,o)&&!r.o(n,o)&&Object.defineProperty(n,o,{enumerable:!0,get:e[o]})},r.o=(n,e)=>Object.prototype.hasOwnProperty.call(n,e),r.r=n=>{"undefined"!=typeof Symbol&&Symbol.toStringTag&&Object.defineProperty(n,Symbol.toStringTag,{value:"Module"}),Object.defineProperty(n,"__esModule",{value:!0})},r(97)})();
return plugin;
}
};