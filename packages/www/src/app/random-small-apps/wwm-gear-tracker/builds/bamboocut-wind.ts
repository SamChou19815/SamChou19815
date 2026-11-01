// 破竹风 (Bamboocut-Wind): 泥犁三垢 (双刀) + 栗子游尘 (绳镖), a 20.8 s rotation.
// A 结算 settles 30% of rows 38, 44, 49 and 54.

import {
  chances,
  elementOf,
  ELEMENTS,
  expectedHit,
  targetOf,
  type BuildDefinition,
  type ElementStats,
} from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./bamboocut-wind.data";

const ROTATION_SECONDS = 20.8;
const SETTLED_ROWS = [38, 44, 49, 54];
const SETTLED_SHARE = 0.3;

function skillDamage(skill: Skill, input: Input): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense;
  const attribute =
    skill.dynamicAttribute === true ? input.primaryAttribute : skill.attribute || "破竹";
  const armor = input.armorSet === "飞隼" ? 1.1 : input.armorSet === "撼天" ? 1.05 : 1;
  // 恶身 ignores 10% of defense.
  const ignored = skill["恶身"] === 1 ? 0.9 : 1;
  const min =
    s.physicalAttackMin * (1 + (skill.extraMinAttackBonus || 0)) * armor -
    defense * ignored +
    (arts.gangluShaobing ? 140 : 0);
  const max =
    Math.max(s.physicalAttackMax * (1 + (skill.extraMaxAttackBonus || 0)), s.physicalAttackMin) *
      armor -
    defense * ignored +
    (arts.gangluShaobing ? 280 : 0);
  const avg = (min + max) * 0.5;
  const pen =
    (s.physicalPenetration + (skill.extraPenetration || 0) + (arts.yishuige ? 10 : 0)) / 200;
  const damageBonus = s.physicalDamageBonus + (skill["特殊"] === "鼠鼠" ? 0.24 : 0);
  const corr = skill.correction || 1;
  const crit =
    Math.min(s.criticalRate + (skill.extraCritRate || 0), 0.8) +
    Math.min(s.directCriticalRate, 0.2);
  const aff =
    Math.min(s.affinityRate + (skill.extraHuiyiRate || 0), 0.4) +
    Math.min(s.directAffinityRate, 0.1);
  const p = chances(crit, aff, Math.min(s.precisionRate, 1));
  const multipliers = {
    glance: 1 + s.scratchDamageBonus,
    crit: 1 + s.critDamageBonus + (skill.extraCritDamage || 0),
    aff: 1 + s.affDamageBonus + (skill.extraHuiyiDamage || 0),
  };

  const physical = (1 + pen) * (1 + damageBonus) * corr;
  let total = expectedHit(p, min, avg, max, skill.multiplier * physical, multipliers);
  const fixed = (skill.fixedDamage || 0) * (1 + (skill.type === "武器" ? s.fixedDamageBonus : 0));
  total += expectedHit(p, fixed, fixed, fixed, physical, multipliers);

  const em = skill.elementMultiplier || 0;
  // 极乐泣血 deals no 属攻 damage.
  if (em <= 0 || skill["特殊"] === "极乐") return total;
  const elementArmor = input.armorSet === "撼天" ? 1.05 : 1;
  // 苦果 adds 10 属攻穿透.
  const fruitPen = skill["苦果"] === 1 ? 10 : 0;
  const elementHit = (element: string, stats: ElementStats) => {
    if (stats.min <= 0 && stats.max <= 0) return 0;
    const base =
      skill.type === "武器" && attribute === element ? 150.7 * (1 + s.fixedDamageBonus) : 0;
    const eMin = stats.min * elementArmor + base;
    const eMax = Math.max(stats.max * elementArmor + base, eMin);
    const scale =
      (attribute === element ? em : skill.multiplier) *
      (1 + (stats.penetration + fruitPen) / 200) *
      (1 + stats.damageBonus);
    return expectedHit(p, eMin, (eMin + eMax) * 0.5, eMax, scale, multipliers);
  };
  for (const element of ELEMENTS) total += elementHit(element, elementOf(s, element));
  if (s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) {
    const own = elementOf(s, attribute);
    const wMin = s.wuxiangAttackMin * elementArmor;
    const wMax = s.wuxiangAttackMax * elementArmor;
    const scale =
      (attribute === "破竹" ? em : skill.multiplier) *
      (1 + (own.penetration + fruitPen) / 200) *
      (1 + own.damageBonus);
    total += expectedHit(p, wMin, (wMin + wMax) * 0.5, wMax, scale, multipliers);
  }
  return total;
}

function dps(input: Input): number {
  const { martialArts: arts, weaponBonus: wb } = input;
  let general = input.generalBonus.general;
  if (!arts.yishuige) general -= 0.05;
  if (arts.zhengrengui) general += 0.08;
  if (arts.tiangong === "毒") general -= 0.005;
  else if (arts.tiangong === "无") general -= 0.015;

  let total = 0;
  const settled = new Map<number, number>();
  for (const row of ROTATION) {
    const skill = SKILLS[row.skillName];
    if (skill == null || row.count === 0 || row.skillName === "结算") continue;
    let bonus = general + (skill.extraDamageBonus || 0);
    if (skill.chargeSkill === 1) bonus += input.generalBonus.charge;
    if (skill.weaponType === "双刀") bonus += wb.dualBlade + wb.allWeapon;
    else if (skill.weaponType === "绳标") bonus += wb.ropeDart + wb.allWeapon;
    const attuned = row.dingyinBonus > 0 ? input.generalBonus.dingyin : 0;
    const damage =
      skillDamage(skill, input) * (1 + bonus) * (skill.correction || 1) * (1 + attuned) * row.count;
    total += damage;
    if (SETTLED_ROWS.includes(row.row)) settled.set(row.row, damage);
  }
  const settlement =
    SETTLED_ROWS.reduce((sum, row) => sum + (settled.get(row) ?? 0), 0) * SETTLED_SHARE;
  if (settlement > 0) total += settlement;
  return total / ROTATION_SECONDS;
}

export const BAMBOOCUT_WIND: BuildDefinition<Input> = {
  id: "bamboocut-wind",
  name: "破竹风",
  baselineKey: "pozhufeng",
  preset: PRESET,
  dps,
  mainWeapon: "dualBlade",
  rawRates: false,
};
