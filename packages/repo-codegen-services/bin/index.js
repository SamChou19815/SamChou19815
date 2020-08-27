module.exports=(()=>{"use strict";var __webpack_modules__={167:(__unused_webpack_module,__webpack_exports__,__webpack_require__)=>{__webpack_require__.d(__webpack_exports__,{A_:()=>createJsonCodegenService});var fs__WEBPACK_IMPORTED_MODULE_0__=__webpack_require__(747);var fs__WEBPACK_IMPORTED_MODULE_0___default=__webpack_require__.n(fs__WEBPACK_IMPORTED_MODULE_0__);var path__WEBPACK_IMPORTED_MODULE_1__=__webpack_require__(622);var path__WEBPACK_IMPORTED_MODULE_1___default=__webpack_require__.n(path__WEBPACK_IMPORTED_MODULE_1__);var typescript__WEBPACK_IMPORTED_MODULE_2__=__webpack_require__(34);var typescript__WEBPACK_IMPORTED_MODULE_2___default=__webpack_require__.n(typescript__WEBPACK_IMPORTED_MODULE_2__);class CodegenInMemoryFilesystem{constructor(e){this.files=new Map(e)}fileExists(e){return this.files.has(e)}readFile(e){const r=this.files.get(e);if(r==null)throw new Error(`No such file: ${e}`);return r}writeFile(e,r){this.files.set(e,r)}deleteFile(e){this.files.delete(e)}}const CodegenRealFilesystem={fileExists:e=>(0,fs__WEBPACK_IMPORTED_MODULE_0__.existsSync)(e),readFile:e=>(0,fs__WEBPACK_IMPORTED_MODULE_0__.readFileSync)(e).toString(),writeFile:(e,r)=>{(0,fs__WEBPACK_IMPORTED_MODULE_0__.mkdirSync)((0,path__WEBPACK_IMPORTED_MODULE_1__.dirname)(e),{recursive:true});(0,fs__WEBPACK_IMPORTED_MODULE_0__.writeFileSync)(e,r)},deleteFile:e=>(0,fs__WEBPACK_IMPORTED_MODULE_0__.unlinkSync)(e)};const createJsonCodegenService=(e,r,t)=>({name:e,sourceFileIsRelevant:r,run:(e,r)=>t(e,JSON.parse(r))});const createTypeScriptCodegenService=(name,sourceFileIsRelevant,run)=>({name:name,sourceFileIsRelevant:sourceFileIsRelevant,run:(sourceFilename,source)=>{const transpiledModuleCode=typescript__WEBPACK_IMPORTED_MODULE_2__.transpile(source,{module:typescript__WEBPACK_IMPORTED_MODULE_2__.ModuleKind.CommonJS});const wrappedModuleCodeForEval=`((exports) => {\n      ${transpiledModuleCode}\n\n      return exports.default;\n    })({})`;const evaluatedSource=eval(wrappedModuleCodeForEval);return run(sourceFilename,evaluatedSource)}})},309:(e,r,t)=>{t.r(r);t.d(r,{default:()=>y});const n=(e,r={})=>({type:"use-action",actionName:e,actionArguments:r});const _=(e,r)=>({type:"run",stepName:e,command:r});const o=e=>{switch(e.type){case"use-action":{const r=`      - uses: ${e.actionName}\n`;if(Object.keys(e.actionArguments).length===0){return r}const t=Object.entries(e.actionArguments).map(([e,r])=>{const t=r.split("\n");if(t.length===1){return`          ${e}: ${t[0]}`}return`          ${e}: |\n${t.map(e=>`            ${e}`).join("\n")}`}).join("\n");return`${r}        with:\n${t}\n`}case"run":{const r=`      - name: ${e.stepName}\n`;const t=e.command.split("\n");if(t.length===1){return`${r}        run: ${t[0]}\n`}return`${r}        run: |\n${t.map(e=>`          ${e}\n`).join("")}`}default:throw new Error}};const a=({jobName:e,jobSteps:r})=>{return`  ${e}:\n    runs-on: ubuntu-latest\n    steps:\n${r.map(o).join("")}`};const s=({workflowName:e,workflowtrigger:{triggerPaths:r,masterBranchOnly:t},workflowSecrets:n=[],workflowJobs:_})=>{const o=`# @generated\n\nname: ${e}\non:\n  push:\n    paths:\n${r.map(e=>`      - '${e}'\n`).join("")}${t?`    branches:\n      - master\n`:""}${n.length>0?`env:\n${n.map(e=>`  ${e}: \${{ secrets.${e} }}`).join("\n")}\n`:""}\njobs:\n${_.map(a).join("")}`;return o};var i=t(747);const c=n("actions/checkout@v2");const l=n("actions/setup-node@v2-beta");const u=n("actions/cache@v2",{path:".yarn/cache\n.pnp.js",key:"yarn-berry-${{ hashFiles('**/yarn.lock') }}","restore-keys":"yarn-berry-"});var p=t(622);const d=[c,l,u,_("Yarn Install","yarn install --immutable")];const m=e=>{const r=r=>{var t,n;return((n=(t=JSON.parse((0,i.readFileSync)((0,p.join)(e.information[r].workspaceLocation,"package.json")).toString()))===null||t===void 0?void 0:t.scripts)===null||n===void 0?void 0:n.deploy)!=null};return Object.fromEntries([...e.topologicallyOrdered.filter(r).map(r=>{const t=`cd-${r}`;return[t,{workflowName:`CD ${r}`,workflowtrigger:{triggerPaths:[...e.information[r].dependencyChain.map(r=>`${e.information[r].workspaceLocation}/**`),"configuration/**",`.github/workflows/generated-*-${r}.yml`],masterBranchOnly:true},workflowSecrets:["FIREBASE_TOKEN"],workflowJobs:[{jobName:"deploy",jobSteps:[...d,_("Build",`yarn workspace ${r} build`),_("Install firebase-tools","sudo npm install -g firebase-tools"),_("Deploy",`yarn workspace ${r} deploy`)]}]}]})])};var w=t(167);const b=()=>["general",{workflowName:"General",workflowtrigger:{triggerPaths:["**"],masterBranchOnly:false},workflowJobs:[{jobName:"lint",jobSteps:[...d,_("Check Codegen","yarn codegen"),_("Check changed","if [[ `git status --porcelain` ]]; then exit 1; fi"),_("Format Check","yarn format:check"),_("Lint","yarn lint")]},{jobName:"build",jobSteps:[n("actions/checkout@v2",{"fetch-depth":"2"}),l,u,_("Yarn Install","yarn install --immutable"),_("Build","yarn compile")]},{jobName:"test",jobSteps:[...d,_("Test","yarn test")]}]}];const f=(0,w.A_)("GitHub Actions Workflows Codegen",e=>e==="workspaces.json",(e,r)=>{return[b(),...Object.entries(m(r))].map(([e,r])=>({outputFilename:`.github/workflows/generated-${e}.yml`,outputContent:s(r)}))});const k={name:"Ignore Files Codegen",sourceFileIsRelevant:e=>e===".gitignore",run:(e,r)=>{const t=`# ${"@"+"generated"}\n\n${r}\n\n# additions\n.yarn\n**/bin/`;return[{outputFilename:".eslintignore",outputContent:t},{outputFilename:".prettierignore",outputContent:t}]}};const E=[f,k];const y=E},747:e=>{e.exports=require("fs")},622:e=>{e.exports=require("path")},34:e=>{e.exports=require("typescript")}};var __webpack_module_cache__={};function __webpack_require__(e){if(__webpack_module_cache__[e]){return __webpack_module_cache__[e].exports}var r=__webpack_module_cache__[e]={exports:{}};var t=true;try{__webpack_modules__[e](r,r.exports,__webpack_require__);t=false}finally{if(t)delete __webpack_module_cache__[e]}return r.exports}(()=>{__webpack_require__.n=(e=>{var r=e&&e.__esModule?()=>e["default"]:()=>e;__webpack_require__.d(r,{a:r});return r})})();(()=>{__webpack_require__.d=((e,r)=>{for(var t in r){if(__webpack_require__.o(r,t)&&!__webpack_require__.o(e,t)){Object.defineProperty(e,t,{enumerable:true,get:r[t]})}}})})();(()=>{__webpack_require__.o=((e,r)=>Object.prototype.hasOwnProperty.call(e,r))})();(()=>{__webpack_require__.r=(e=>{if(typeof Symbol!=="undefined"&&Symbol.toStringTag){Object.defineProperty(e,Symbol.toStringTag,{value:"Module"})}Object.defineProperty(e,"__esModule",{value:true})})})();__webpack_require__.ab=__dirname+"/";return __webpack_require__(309)})();