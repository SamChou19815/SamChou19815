// 裂石钧 (Stonesplit-Scale): 陌刀 + 唐横刀, a 21.5 s rotation.

import { chances, expectedHit, targetOf, type BuildDefinition } from "./common";
import {
  ADAPTIVE_STEEL,
  PRESET,
  ROTATION,
  SKILLS,
  THROAT_PIERCE,
  type Input,
  type Skill,
} from "./stonesplit-scale.data";

const ROTATION_SECONDS = 21.5;

function skillDamage(name: string, skill: Skill, input: Input): number {
  const s = input.panelAttributes;
  const defense = targetOf(input.targetType).defense;
  const armor = input.armorSet === "飞隼" ? 1.1 : 1;
  const food = input.buffs.gangluShaobing;
  let critDamage = skill.extraCritDamage || 0;
  let penetration = skill.extraPenetration || 0;
  // Skills whose coefficients assume 穿喉诀 lose its bonus without it.
  const throat = THROAT_PIERCE[name];
  if (throat != null && !input.martialArts.chuanhoujue) {
    critDamage -= throat[0] ?? 0;
    penetration -= throat[1] ?? 0;
  }
  const adaptive = ADAPTIVE_STEEL[name];
  if (adaptive != null && input.martialArts.yijingyiwu) critDamage += adaptive;

  const min =
    s.physicalAttackMin * (1 + (skill.extraMinAttackBonus || 0)) * armor -
    defense +
    (food ? 140 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + (skill.extraMaxAttackBonus || 0)), s.physicalAttackMin) *
      armor -
    defense +
    (food ? 280 : 0);
  const avg = (min + max) * 0.5;
  const pen = (s.physicalPenetration + penetration + (input.martialArts.yishuige ? 10 : 0)) / 200;
  const corr = skill.correction || 1;

  let crit: number;
  let aff: number;
  let precision: number;
  if (skill.forceCrit === 1) {
    crit = 1;
    aff = 0;
    precision = 1;
  } else {
    crit =
      Math.min(s.criticalRate + (skill.extraCritRate || 0), 0.8) +
      Math.min(s.directCriticalRate, 0.2);
    aff =
      Math.min(s.affinityRate + (skill.extraHuiyiRate || 0), 0.4) +
      Math.min(s.directAffinityRate, 0.1);
    precision = Math.min(s.precisionRate, 1);
  }
  const p = chances(crit, aff, precision, skill.weaponType === "陌刀");
  const affDamage = skill.extraHuiyiDamage || 0;
  const multipliers = {
    glance: 1 + s.scratchDamageBonus,
    crit: 1 + s.critDamageBonus + critDamage,
    aff: 1 + s.affDamageBonus + affDamage,
  };
  const physical = skill.multiplier * (1 + pen) * (1 + s.physicalDamageBonus) * corr;
  let total = expectedHit(p, min, avg, max, physical, multipliers);
  const fixed = (skill.fixedDamage || 0) * (1 + (skill.type === "武器" ? s.fixedDamageBonus : 0));
  total += expectedHit(
    p,
    fixed,
    fixed,
    fixed,
    (1 + pen) * (1 + s.physicalDamageBonus) * corr,
    multipliers,
  );

  const em = skill.elementMultiplier || 0;
  const mark = skill.yishuigeMark === 1 && input.martialArts.yishuige ? 8 : 0;
  const weaponBase = 150.7 * (1 + s.fixedDamageBonus);
  if ((s.stonesplitAttackMin > 0 || s.stonesplitAttackMax > 0) && em > 0) {
    const own = skill.attribute === "裂石";
    const base = skill.type === "武器" && own ? weaponBase : 0;
    const eMin = s.stonesplitAttackMin + base;
    const eMax = Math.max(s.stonesplitAttackMax + base, eMin);
    const ePen = (s.stonesplitPenetration + (own ? s.wuxiangPenetration : 0) + mark) / 200 + 0.06;
    const scale = (own ? em : skill.multiplier) * (1 + ePen) * (1 + s.stonesplitDamageBonus);
    total += expectedHit(p, eMin, (eMin + eMax) * 0.5, eMax, scale, multipliers);
  }
  if ((s.bellstrikeAttackMin > 0 || s.bellstrikeAttackMax > 0) && em > 0) {
    const own = skill.attribute === "鸣金";
    const base = skill.type === "武器" && own ? weaponBase : 0;
    const eMin = s.bellstrikeAttackMin + base;
    const eMax = Math.max(s.bellstrikeAttackMax + base, eMin);
    const ePen = (s.bellstrikePenetration + (own ? s.wuxiangPenetration : 0) + mark) / 200;
    const scale = (own ? em : skill.multiplier) * (1 + ePen) * (1 + s.bellstrikeDamageBonus);
    total += expectedHit(p, eMin, (eMin + eMax) * 0.5, eMax, scale, multipliers);
  }
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    let ePen = 0;
    let eDmg = 0;
    if (skill.attribute === "裂石") {
      ePen = s.stonesplitPenetration + 12 + s.wuxiangPenetration + mark;
      eDmg = s.stonesplitDamageBonus;
    } else if (skill.attribute === "鸣金") {
      ePen = s.bellstrikePenetration + s.wuxiangPenetration + mark;
      eDmg = s.bellstrikeDamageBonus;
    }
    const scale =
      (skill.attribute === "裂石" ? em : skill.multiplier) * (1 + ePen / 200) * (1 + eDmg);
    const { wuxiangAttackMin: wMin, wuxiangAttackMax: wMax } = s;
    total += expectedHit(p, wMin, (wMin + wMax) * 0.5, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  let general = input.generalBonus.general;
  if (!input.martialArts.yishuige) general -= 0.05;
  if (input.martialArts.zhengrengui) general += 0.08;
  const { weaponBonus: wb } = input;
  let total = 0;
  for (const { skillName, count, dingyinBonus } of ROTATION) {
    const skill = SKILLS[skillName];
    if (skill == null || count === 0) continue;
    let bonus = general + (skill.extraDamageBonus || 0);
    if (skill.chargeSkill === 1) bonus += input.generalBonus.charge;
    if (skill.weaponType === "陌刀") bonus += wb.modao + wb.allWeapon;
    else if (skill.weaponType === "横刀") bonus += wb.hengdao + wb.allWeapon;
    else if (skill.weaponType !== "N/A" && skill.weaponType !== "") bonus += wb.allWeapon;
    const attuned = dingyinBonus > 0 ? input.generalBonus.dingyin : 0;
    total +=
      skillDamage(skillName, skill, input) *
      (1 + bonus) *
      (skill.correction || 1) *
      (1 + attuned) *
      count;
  }
  return total / ROTATION_SECONDS;
}

export const STONESPLIT_SCALE: BuildDefinition<Input> = {
  id: "stonesplit-scale",
  name: "裂石钧",
  baselineKey: "lieshijun",
  preset: PRESET,
  dps,
  mainWeapon: "modao",
  rawRates: false,
};
