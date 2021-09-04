// @generated
/* eslint-disable */
//prettier-ignore
module.exports = {
name: "@yarnpkg/plugin-monorail",
factory: function (require) {
var plugin=(()=>{var N=Object.create;var m=Object.defineProperty,B=Object.defineProperties,M=Object.getOwnPropertyDescriptor,G=Object.getOwnPropertyDescriptors,H=Object.getOwnPropertyNames,f=Object.getOwnPropertySymbols,_=Object.getPrototypeOf,k=Object.prototype.hasOwnProperty,w=Object.prototype.propertyIsEnumerable;var W=(e,n,o)=>n in e?m(e,n,{enumerable:!0,configurable:!0,writable:!0,value:o}):e[n]=o,F=(e,n)=>{for(var o in n||(n={}))k.call(n,o)&&W(e,o,n[o]);if(f)for(var o of f(n))w.call(n,o)&&W(e,o,n[o]);return e},x=(e,n)=>B(e,G(n)),Y=e=>m(e,"__esModule",{value:!0});var u=e=>{if(typeof require!="undefined")return require(e);throw new Error('Dynamic require of "'+e+'" is not supported')};var b=(e,n)=>{var o={};for(var r in e)k.call(e,r)&&n.indexOf(r)<0&&(o[r]=e[r]);if(e!=null&&f)for(var r of f(e))n.indexOf(r)<0&&w.call(e,r)&&(o[r]=e[r]);return o};var q=(e,n)=>{Y(e);for(var o in n)m(e,o,{get:n[o],enumerable:!0})},U=(e,n,o)=>{if(n&&typeof n=="object"||typeof n=="function")for(let r of H(n))!k.call(e,r)&&r!=="default"&&m(e,r,{get:()=>n[r],enumerable:!(o=M(n,r))||o.enumerable});return e},g=e=>U(Y(m(e!=null?N(_(e)):{},"default",e&&e.__esModule&&"default"in e?{get:()=>e.default,enumerable:!0}:{value:e,enumerable:!0})),e);var te={};q(te,{default:()=>re});var v=g(u("fs")),O=g(u("clipanion"));var y=g(u("child_process")),D=g(u("fs")),h=g(u("path"));var d=e=>process.stderr.isTTY?n=>`[${e}m${n}[0m`:n=>n,A=d(31),L=d(32),T=d(33),P=d(34),I=d(35),ie=d(36);var V=["\u280B","\u2819","\u2839","\u2838","\u283C","\u2834","\u2826","\u2827","\u2807","\u280F"],z=e=>{let n=0,o=new Date().getTime();return setInterval(()=>{let r=`${((new Date().getTime()-o)/1e3).toFixed(1)}s`,s=e(r),c=V[n%10];process.stderr.write(T(`${s} ${c}\r`)),n+=1},process.stderr.isTTY?40:1e3)},$=z;var K=e=>{let n=(o,r)=>{let s=(0,y.spawnSync)("git",["diff",o,...r?[r]:[],"--name-only","--",e]).stdout.toString().trim();return s===""?[]:s.split(`
`)};return process.env.CI?n("HEAD^","HEAD"):n("origin/main")},Q=e=>JSON.parse((0,D.readFileSync)(e).toString()),X=(e,n)=>{var s,c;let o=i=>{var t,a;return(0,h.dirname)(i)!==(0,h.join)((a=(t=e.information[n])==null?void 0:t.workspaceLocation)!=null?a:".","bin")};return((c=(s=e.information[n])==null?void 0:s.dependencyChain)!=null?c:[]).some(i=>{var l,p;let t=(p=(l=e.information[i])==null?void 0:l.workspaceLocation)!=null?p:".";return K(t).some(o)})},Z=e=>e.topologicallyOrdered.map(n=>{let o=X(e,n);return[n,o]}).filter(([,n])=>n).map(([n])=>n),ee=async()=>{let e=Q("workspaces.json"),n=Z(e);n.forEach(t=>{console.error(P(`[i] \`${t}\` needs to be recompiled.`))});let o=$(t=>`[?] Compiling (${t})`),s=await Promise.all(n.map(t=>{let a=(0,y.spawn)("yarn",["workspace",t,"compile"],{shell:!0,stdio:["ignore","pipe","ignore"]}),l="";return a.stdout.on("data",p=>{l+=p.toString()}),new Promise(p=>{a.on("exit",S=>p([t,S===0,l]))})}));clearInterval(o);let c=s.map(t=>t[2]).join(""),i=s.filter(t=>!t[1]).map(t=>t[0]);return i.length===0?(console.error(L("[\u2713] All workspaces have been successfully compiled!")),!0):(console.error(I("[!] Compilation finished with some errors.")),console.error(c.trim()),i.forEach(t=>{console.error(A(`[x] \`${t}\` failed to exit with 0.`))}),!1)},R=ee;function j(e){return e.scope==null?e.name:`@${e.scope}/${e.name}`}function ne(e){let n=new Map;return e.workspaces.forEach(o=>{let r=o.relativeCwd;if(r===".")return;let s=j(o.locator),c=Array.from(o.getRecursiveWorkspaceDependencies()).map(i=>j(i.locator));n.set(s,{workspaceLocation:r,dependencies:c})}),n}function J(e,n){let o=[],r=[],s=new Set,c=new Set;function i(t){var l;if(c.has(t)){if(!s.has(t))return;r.push(t);let p=r.indexOf(t),S=r.slice(p,r.length).join(" -> ");throw new Error(`Cyclic dependency detected: ${S}`)}let a=(l=e.get(t))==null?void 0:l.dependencies;if(a==null)throw new Error(`Workspace ${n} is not found!`);c.add(t),r.push(t),s.add(t),a.forEach(i),s.delete(t),r.pop(),o.push(t)}return i(n),o}function C(e){let n=ne(e);return{__type__:"@generated",information:Object.fromEntries(Array.from(n.entries()).map(c=>{var[o,i]=c,t=i,{dependencies:r}=t,s=b(t,["dependencies"]);return[o,x(F({},s),{dependencyChain:J(n,o)})]}).sort(([o],[r])=>o.localeCompare(r))),topologicallyOrdered:(()=>{let o=[],r=new Set;return Array.from(n.keys()).forEach(s=>{J(n,s).forEach(i=>{r.has(i)||(o.push(i),r.add(i))})}),o})()}}var E=class extends O.Command{async execute(){return await R()?0:1}};E.paths=[["c"]];var oe={hooks:{afterAllInstalled(e){let n=C(e);(0,v.writeFileSync)("workspaces.json",`${JSON.stringify(n,void 0,2)}
`)}},commands:[E]},re=oe;return te;})();
return plugin;
}
};
