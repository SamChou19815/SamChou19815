// 裂石威 (Stonesplit-Might): 陌刀 + 枪, a 28.5 s rotation.

import { chances, expectedHit, targetOf, type BuildDefinition } from "./common";
import { PRESET, ROTATION, SKILLS, type Input, type Skill } from "./stonesplit-might.data";

const ROTATION_SECONDS = 28.5;

function skillDamage(skill: Skill, input: Input): number {
  const s = input.panelAttributes;
  const { martialArts: arts } = input;
  const defense = targetOf(input.targetType).defense;
  const armor = input.armorSet === "飞隼" ? 1.1 : 1;
  // The 陌刀 talent adds 60 外功攻击 to the skills that carry it.
  const talent = skill.special === "陌刀天赋" ? 60 : 0;
  const min =
    (s.physicalAttackMin + talent) * (1 + (skill.extraMinAttackBonus || 0)) * armor -
    defense +
    (arts.gangluShaobing ? 140 : 0);
  const max =
    Math.max(
      (s.physicalAttackMax + talent) * (1 + (skill.extraMaxAttackBonus || 0)),
      s.physicalAttackMin + talent,
    ) *
      armor -
    defense +
    (arts.gangluShaobing ? 280 : 0);
  const avg = (min + max) * 0.5;
  const pen =
    (s.physicalPenetration +
      (skill.extraPenetration || 0) +
      (arts.duanshizhigou ? 10 : 0) +
      (arts.yishuige ? 10 : 0)) /
    200;
  const corr = skill.correction || 1;
  const crit =
    Math.min(s.criticalRate + (skill.extraCritRate || 0), 0.8) +
    Math.min(s.directCriticalRate, 0.2);
  const aff =
    Math.min(s.affinityRate + (skill.extraHuiyiRate || 0), 0.4) +
    Math.min(s.directAffinityRate, 0.1);
  const p = chances(crit, aff, Math.min(s.precisionRate, 1), skill.weaponType === "陌刀");
  const multipliers = {
    glance: 1 + s.scratchDamageBonus,
    crit:
      1 + s.critDamageBonus + (input.armorSet === "时雨" ? 0.1 : 0) + (skill.extraCritDamage || 0),
    aff: 1 + s.affDamageBonus + (skill.extraHuiyiDamage || 0),
  };
  let total = expectedHit(
    p,
    min,
    avg,
    max,
    skill.multiplier * (1 + pen) * (1 + s.physicalDamageBonus) * corr,
    multipliers,
  );
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
  const weaponBase = 150.7 * (1 + s.fixedDamageBonus);
  for (const [element, eMinRaw, eMaxRaw, ePen, eDmg] of [
    [
      "裂石",
      s.stonesplitAttackMin,
      s.stonesplitAttackMax,
      s.stonesplitPenetration,
      s.stonesplitDamageBonus,
    ],
    [
      "鸣金",
      s.bellstrikeAttackMin,
      s.bellstrikeAttackMax,
      s.bellstrikePenetration,
      s.bellstrikeDamageBonus,
    ],
  ] as const) {
    if (!((eMinRaw > 0 || eMaxRaw > 0) && em > 0)) continue;
    const own = skill.attribute === element;
    const base = skill.type === "武器" && own ? weaponBase : 0;
    const eMin = eMinRaw + base;
    const eMax = Math.max(eMaxRaw + base, eMin);
    const scale = (own ? em : skill.multiplier) * (1 + ePen / 200) * (1 + eDmg);
    total += expectedHit(p, eMin, (eMin + eMax) * 0.5, eMax, scale, multipliers);
  }
  if ((s.wuxiangAttackMin > 0 || s.wuxiangAttackMax > 0) && em > 0) {
    let ePen = 0;
    let eDmg = 0;
    if (skill.attribute === "裂石") {
      ePen = s.stonesplitPenetration;
      eDmg = s.stonesplitDamageBonus;
    } else if (skill.attribute === "鸣金") {
      ePen = s.bellstrikePenetration;
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
  if (input.martialArts.kangzaodafa) general += 0.1;
  const { weaponBonus: wb } = input;
  let total = 0;
  for (const { skillName, count, dingyinBonus } of ROTATION) {
    const skill = SKILLS[skillName];
    if (skill == null || count === 0) continue;
    let bonus = general + (skill.extraDamageBonus || 0);
    if (skill.chargeSkill === 1) bonus += input.generalBonus.charge;
    if (skill.weaponType === "陌刀") bonus += wb.modao + wb.allWeapon;
    else if (skill.weaponType === "枪") bonus += wb.qiang + wb.allWeapon;
    const attuned = dingyinBonus > 0 ? input.generalBonus.dingyin : 0;
    total +=
      skillDamage(skill, input) * (1 + bonus) * (skill.correction || 1) * (1 + attuned) * count;
  }
  return total / ROTATION_SECONDS;
}

export const STONESPLIT_MIGHT: BuildDefinition<Input> = {
  id: "stonesplit-might",
  name: "裂石威",
  baselineKey: "lieshiwei",
  preset: PRESET,
  dps,
  mainWeapon: "modao",
  rawRates: false,
};
