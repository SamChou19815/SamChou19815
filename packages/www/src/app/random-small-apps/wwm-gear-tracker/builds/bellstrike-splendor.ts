// 鸣金虹 (Bellstrike-Splendor): 无名剑 + 无名枪, a 55 s rotation.

import { chances, expectedHit, isOne, targetOf, type BuildDefinition } from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./bellstrike-splendor.data";

const ROTATION_SECONDS = 55;
// This calculator fixes crit and affinity damage instead of reading the panel.
const CRIT_DAMAGE = 1.5;
const AFF_DAMAGE = 1.35 + 0.052;
// …and the 鸣金 damage bonus, and 武器 skills' 固定伤害加成.
const BELLSTRIKE_BONUS = 1 + 0.118;
const FIXED_BONUS = 0.0725;

function skillDamage(name: string, skill: Skill, input: Input): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense;
  const armor = input.armorSet === "飞隼" ? 1.1 : 1;
  const min =
    s.physicalAttackMin * (1 + skill.extraMinPhysical) * armor -
    defense +
    (arts.ganglushaobing ? 140 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + skill.extraMaxPhysical) * armor, min + defense) -
    defense +
    (arts.ganglushaobing ? 280 : 0);
  const avg = (min + max) / 2;
  const pen =
    (s.physicalPenetration +
      (skill.extraPenetration || 0) +
      (arts.yishuige ? 10 : 0) +
      (arts.duanshizhigou ? 10 : 0)) /
    200;
  const corr = skill.correction || 1;
  // The table's extraAffRate / extraCritRate columns are read crossed, as the site does.
  const crit =
    Math.min(s.criticalRate + skill.extraAffRate, 0.8) + Math.min(s.directCriticalRate, 0.2);
  // 长风 adds affinity, more with 玉斗.
  let gale = 0;
  if (isOne(skill.changfeng)) {
    if (input.armorSet === "玉斗") gale = 0.105;
    else if (input.armorSet !== "其他") gale = 0.03;
  }
  const aff = Math.min(s.affinityRate + skill.extraCritRate, 0.4) + s.directAffinityRate + gale;
  const p = chances(crit, aff, Math.min(s.precisionRate, 1));
  let affDamage = skill.extraAffDamage;
  if ((name.includes("笛") || name.includes("长风")) && input.armorSet !== "玉斗") affDamage -= 0.1;
  if (name.includes("蚀渊") && input.shiyuantie) affDamage += 0.1;
  const multipliers = {
    glance: 1,
    crit: CRIT_DAMAGE + skill.extraCritDamage,
    aff: AFF_DAMAGE + affDamage,
  };

  let total = expectedHit(p, min, avg, max, skill.multiplier * (1 + pen) * corr, multipliers);
  const fixed = skill.fixedDamage * (1 + (skill.skillType === "武器" ? FIXED_BONUS : 0));
  total += expectedHit(p, fixed, fixed, fixed, (1 + pen) * corr, multipliers);

  const em = skill.elementMultiplier;
  if (skill.element === "鸣金" && em > 0) {
    const base = skill.skillType === "武器" ? 150.7 * 1.0725 : 0;
    const eMin = s.bellstrikeAttackMin + base;
    const eMax = Math.max(s.bellstrikeAttackMax + base, eMin);
    const scale = em * (1 + s.bellstrikePenetration / 200) * BELLSTRIKE_BONUS;
    total += expectedHit(p, eMin, (eMin + eMax) / 2, eMax, scale, multipliers);
  }
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    const ePen = (skill.element === "鸣金" ? s.bellstrikePenetration : 0) + s.wuxiangPenetration;
    const scale = em * (1 + ePen / 200) * BELLSTRIKE_BONUS;
    const { wuxiangAttackMin: wMin, wuxiangAttackMax: wMax } = s;
    total += expectedHit(p, wMin, (wMin + wMax) / 2, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  const { martialArts: arts, weaponBonus: wb } = input;
  let general = targetOf(input.targetType).generalBonus;
  if (arts.yishuige) general += 0.05;
  if (arts.tiangong === "火") general += 0.015;
  else if (arts.tiangong === "毒") general += 0.01;
  general += wb.boss;
  let total = 0;
  for (const { name, count } of ROTATION) {
    const skill = SKILLS[name];
    if (skill == null) continue;
    const charged = isOne(skill.isCharge);
    let bonus = skill.extraBonus;
    // Rows tagged with 威猛 assume 威猛歌.
    if (!arts.weimengge) {
      if (name.includes("10%威猛")) bonus -= 0.1;
      else if (name.includes("5%威猛")) bonus -= 0.09;
      else if (name.includes("2%威猛")) bonus -= 0.02;
    }
    if (charged) bonus += input.generalBonus.charge;
    let weapon = 0;
    if (skill.weaponType === "剑") weapon = wb.sword + wb.allWeapon;
    else if (skill.weaponType === "枪") weapon = wb.spear + wb.allWeapon;
    else if (
      skill.weaponType === "N/A" ||
      skill.skillType === "奇术" ||
      skill.skillType === "心法"
    ) {
      weapon = wb.na;
      if (skill.skillType === "奇术") {
        weapon +=
          name.includes("AOE") || name.includes("群")
            ? input.skillBonus.aoe
            : input.skillBonus.singleTarget;
      }
    }
    const attuned = charged ? input.tuningBonus.swordCharge : 0;
    total +=
      skillDamage(name, skill, input) *
      (1 + (general + bonus + weapon)) *
      (skill.correction || 1) *
      (1 + attuned) *
      count;
  }
  return total / ROTATION_SECONDS;
}

export const BELLSTRIKE_SPLENDOR: BuildDefinition<Input> = {
  id: "bellstrike-splendor",
  name: "鸣金虹",
  baselineKey: "mingjinhong",
  preset: PRESET,
  dps,
  mainWeapon: "sword",
  rawRates: false,
};
