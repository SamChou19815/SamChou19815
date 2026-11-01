// 破竹鸢 (Bamboocut-Kite): 天志垂象 (手甲) + 千机索天 (绳镖), a 55.5 s rotation.
// Its average hit is (min + max) × fistExpectation rather than the midpoint.

import {
  chances,
  elementOf,
  ELEMENTS,
  expectedHit,
  targetOf,
  type BuildDefinition,
  type Chances,
  type ElementStats,
} from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./bamboocut-kite.data";

const ROTATION_SECONDS = 55.5;
const CRIT_DAMAGE = 1.54;
const AFF_DAMAGE = 1.35;
const FIXED_BONUS = 0.0725;

function skillDamage(
  skill: Skill,
  input: Input,
  yishuigePenetration: number,
  critShift: number,
  sanqiong: boolean,
): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense;
  const attribute =
    skill.dynamicAttribute === true ? input.primaryAttribute : skill.attribute || "破竹";
  const armor = input.armorSet === "飞隼" ? 1.1 : input.armorSet === "撼天" ? 1.05 : 1;
  const min =
    s.physicalAttackMin * (1 + (skill.extraMinAttackBonus || 0)) * armor -
    defense +
    (arts.gangluShaobing ? 140 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + (skill.extraMaxAttackBonus || 0)), s.physicalAttackMin) *
      armor -
    defense +
    (arts.gangluShaobing ? 280 : 0);
  const expectation = input.fistExpectation;
  const avg = (min + max) * expectation;
  const penetration =
    (skill.extraPenetration || 0) +
    (arts.yishuige ? yishuigePenetration : 0) +
    (sanqiong && skill.special === 1 && skill.sanqiong !== 1 ? 20 : 0);
  const pen = (s.physicalPenetration + penetration) / 200;
  const fistBonus = skill.fistFlag === 1 ? 0 : 0.09;
  const damageBonus = s.physicalDamageBonus + fistBonus + (sanqiong ? 0.025 : 0);
  const corr = skill.correction || 1;
  const crit =
    Math.min(s.criticalRate + (skill.extraCritRate || 0) + critShift, 0.8) +
    Math.min(s.directCriticalRate, 0.2);
  const aff =
    Math.min(s.affinityRate + (skill.extraHuiyiRate || 0), 0.4) +
    Math.min(s.directAffinityRate, 0.1);
  const p: Chances = chances(crit, aff, Math.min(s.precisionRate, 1));
  const multipliers = {
    glance: 1,
    crit: CRIT_DAMAGE + (skill.extraCritDamage || 0),
    aff: AFF_DAMAGE + (skill.extraHuiyiDamage || 0),
  };

  const physical = (1 + pen) * (1 + damageBonus) * corr;
  let total = expectedHit(p, min, avg, max, skill.multiplier * physical, multipliers);
  const fixed = skill.fixedDamage * (1 + (skill.type === "武器" ? FIXED_BONUS : 0));
  total += expectedHit(p, fixed, fixed, fixed, physical, multipliers);

  const em = skill.elementMultiplier;
  const specialPen = skill.special === 1 ? 4 : 0;
  const sanqiongPen = skill.sanqiong === 1 ? 20 : skill.special === 1 && sanqiong ? 20 : 0;
  const elementHit = (element: string, stats: ElementStats) => {
    if ((stats.min <= 0 && stats.max <= 0) || em <= 0) return 0;
    const base = skill.type === "武器" && attribute === element ? 150.7 * 1.0725 : 0;
    const eMin = stats.min * armor + base;
    const eMax = Math.max(stats.max * armor + base, eMin);
    const scale =
      (attribute === element ? em : skill.multiplier) *
      (1 + (stats.penetration + specialPen + sanqiongPen) / 200) *
      (1 + (stats.damageBonus + fistBonus));
    return expectedHit(p, eMin, (eMin + eMax) * expectation, eMax, scale, multipliers);
  };
  for (const element of ELEMENTS) total += elementHit(element, elementOf(s, element));
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    const own = elementOf(s, attribute);
    const wMin = s.wuxiangAttackMin * armor;
    const wMax = s.wuxiangAttackMax * armor;
    const scale =
      (attribute === "破竹" ? em : skill.multiplier) *
      (1 + (own.penetration + specialPen + sanqiongPen) / 200) *
      (1 + (own.damageBonus + fistBonus));
    total += expectedHit(p, wMin, (wMin + wMax) * expectation, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  const { martialArts: arts, weaponBonus: wb } = input;
  let general = input.generalBonus.general;
  if (!arts.yishuige) general -= 0.05;
  if (arts.tiangong === "毒") general -= 0.005;
  else if (arts.tiangong === "无") general -= 0.015;
  // The panel's crit includes 断石之构's and 易水歌's direct crit; take it off without them.
  let critShift = 0;
  if (!arts.duanshizhigou) critShift -= 0.041;
  if (!arts.yishuige) critShift -= 0.046;

  let total = 0;
  for (const row of ROTATION) {
    const skill = SKILLS[row.skillName];
    if (skill == null || row.count === 0) continue;
    const damage = skillDamage(
      skill,
      input,
      row.yishuigePenetration ?? 0,
      critShift,
      arts.sanqiongzhizhi,
    );
    let bonus = general + (skill.extraDamageBonus || 0);
    if (skill.chargeSkill === 1) bonus += input.generalBonus.charge;
    if (skill.weaponType === "拳甲") bonus += wb.fist + wb.allWeapon;
    else if (skill.weaponType === "绳标") bonus += wb.ropeDart + wb.allWeapon;
    if (arts.yishuige && row.yishuigePenalty) bonus -= row.yishuigePenalty;
    if (input.chenghuantie && row.chenghuantieBonus) bonus += 0.2;
    const attuned = row.dingyinBonus > 0 ? input.generalBonus.fistCharge : 0;
    total += damage * (1 + bonus) * (skill.correction || 1) * (1 + attuned) * row.count;
  }
  return total / ROTATION_SECONDS;
}

export const BAMBOOCUT_KITE: BuildDefinition<Input> = {
  id: "bamboocut-kite",
  name: "破竹鸢",
  baselineKey: "pozhuyuan",
  preset: PRESET,
  dps,
  mainWeapon: "fist",
  rawRates: false,
};
