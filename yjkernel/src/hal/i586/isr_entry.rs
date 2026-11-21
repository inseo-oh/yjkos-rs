// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use core::arch::global_asm;

unsafe extern "C" {
    pub(super) fn hal_i586_isr_exception0_entry();
    pub(super) fn hal_i586_isr_exception1_entry();
    pub(super) fn hal_i586_isr_exception2_entry();
    pub(super) fn hal_i586_isr_exception3_entry();
    pub(super) fn hal_i586_isr_exception4_entry();
    pub(super) fn hal_i586_isr_exception5_entry();
    pub(super) fn hal_i586_isr_exception6_entry();
    pub(super) fn hal_i586_isr_exception7_entry();
    pub(super) fn hal_i586_isr_exception8_entry();
    pub(super) fn hal_i586_isr_exception9_entry();
    pub(super) fn hal_i586_isr_exception10_entry();
    pub(super) fn hal_i586_isr_exception11_entry();
    pub(super) fn hal_i586_isr_exception12_entry();
    pub(super) fn hal_i586_isr_exception13_entry();
    pub(super) fn hal_i586_isr_exception14_entry();
    pub(super) fn hal_i586_isr_exception15_entry();
    pub(super) fn hal_i586_isr_exception16_entry();
    pub(super) fn hal_i586_isr_exception17_entry();
    pub(super) fn hal_i586_isr_exception18_entry();
    pub(super) fn hal_i586_isr_exception19_entry();
    pub(super) fn hal_i586_isr_exception20_entry();
    pub(super) fn hal_i586_isr_exception21_entry();
    pub(super) fn hal_i586_isr_exception22_entry();
    pub(super) fn hal_i586_isr_exception23_entry();
    pub(super) fn hal_i586_isr_exception24_entry();
    pub(super) fn hal_i586_isr_exception25_entry();
    pub(super) fn hal_i586_isr_exception26_entry();
    pub(super) fn hal_i586_isr_exception27_entry();
    pub(super) fn hal_i586_isr_exception28_entry();
    pub(super) fn hal_i586_isr_exception29_entry();
    pub(super) fn hal_i586_isr_exception30_entry();
    pub(super) fn hal_i586_isr_exception31_entry();
    pub(super) fn hal_i586_isr_interrupt32_entry();
    pub(super) fn hal_i586_isr_interrupt33_entry();
    pub(super) fn hal_i586_isr_interrupt34_entry();
    pub(super) fn hal_i586_isr_interrupt35_entry();
    pub(super) fn hal_i586_isr_interrupt36_entry();
    pub(super) fn hal_i586_isr_interrupt37_entry();
    pub(super) fn hal_i586_isr_interrupt38_entry();
    pub(super) fn hal_i586_isr_interrupt39_entry();
    pub(super) fn hal_i586_isr_interrupt40_entry();
    pub(super) fn hal_i586_isr_interrupt41_entry();
    pub(super) fn hal_i586_isr_interrupt42_entry();
    pub(super) fn hal_i586_isr_interrupt43_entry();
    pub(super) fn hal_i586_isr_interrupt44_entry();
    pub(super) fn hal_i586_isr_interrupt45_entry();
    pub(super) fn hal_i586_isr_interrupt46_entry();
    pub(super) fn hal_i586_isr_interrupt47_entry();
    pub(super) fn hal_i586_isr_interrupt48_entry();
    pub(super) fn hal_i586_isr_interrupt49_entry();
    pub(super) fn hal_i586_isr_interrupt50_entry();
    pub(super) fn hal_i586_isr_interrupt51_entry();
    pub(super) fn hal_i586_isr_interrupt52_entry();
    pub(super) fn hal_i586_isr_interrupt53_entry();
    pub(super) fn hal_i586_isr_interrupt54_entry();
    pub(super) fn hal_i586_isr_interrupt55_entry();
    pub(super) fn hal_i586_isr_interrupt56_entry();
    pub(super) fn hal_i586_isr_interrupt57_entry();
    pub(super) fn hal_i586_isr_interrupt58_entry();
    pub(super) fn hal_i586_isr_interrupt59_entry();
    pub(super) fn hal_i586_isr_interrupt60_entry();
    pub(super) fn hal_i586_isr_interrupt61_entry();
    pub(super) fn hal_i586_isr_interrupt62_entry();
    pub(super) fn hal_i586_isr_interrupt63_entry();
    pub(super) fn hal_i586_isr_interrupt64_entry();
    pub(super) fn hal_i586_isr_interrupt65_entry();
    pub(super) fn hal_i586_isr_interrupt66_entry();
    pub(super) fn hal_i586_isr_interrupt67_entry();
    pub(super) fn hal_i586_isr_interrupt68_entry();
    pub(super) fn hal_i586_isr_interrupt69_entry();
    pub(super) fn hal_i586_isr_interrupt70_entry();
    pub(super) fn hal_i586_isr_interrupt71_entry();
    pub(super) fn hal_i586_isr_interrupt72_entry();
    pub(super) fn hal_i586_isr_interrupt73_entry();
    pub(super) fn hal_i586_isr_interrupt74_entry();
    pub(super) fn hal_i586_isr_interrupt75_entry();
    pub(super) fn hal_i586_isr_interrupt76_entry();
    pub(super) fn hal_i586_isr_interrupt77_entry();
    pub(super) fn hal_i586_isr_interrupt78_entry();
    pub(super) fn hal_i586_isr_interrupt79_entry();
    pub(super) fn hal_i586_isr_interrupt80_entry();
    pub(super) fn hal_i586_isr_interrupt81_entry();
    pub(super) fn hal_i586_isr_interrupt82_entry();
    pub(super) fn hal_i586_isr_interrupt83_entry();
    pub(super) fn hal_i586_isr_interrupt84_entry();
    pub(super) fn hal_i586_isr_interrupt85_entry();
    pub(super) fn hal_i586_isr_interrupt86_entry();
    pub(super) fn hal_i586_isr_interrupt87_entry();
    pub(super) fn hal_i586_isr_interrupt88_entry();
    pub(super) fn hal_i586_isr_interrupt89_entry();
    pub(super) fn hal_i586_isr_interrupt90_entry();
    pub(super) fn hal_i586_isr_interrupt91_entry();
    pub(super) fn hal_i586_isr_interrupt92_entry();
    pub(super) fn hal_i586_isr_interrupt93_entry();
    pub(super) fn hal_i586_isr_interrupt94_entry();
    pub(super) fn hal_i586_isr_interrupt95_entry();
    pub(super) fn hal_i586_isr_interrupt96_entry();
    pub(super) fn hal_i586_isr_interrupt97_entry();
    pub(super) fn hal_i586_isr_interrupt98_entry();
    pub(super) fn hal_i586_isr_interrupt99_entry();
    pub(super) fn hal_i586_isr_interrupt100_entry();
    pub(super) fn hal_i586_isr_interrupt101_entry();
    pub(super) fn hal_i586_isr_interrupt102_entry();
    pub(super) fn hal_i586_isr_interrupt103_entry();
    pub(super) fn hal_i586_isr_interrupt104_entry();
    pub(super) fn hal_i586_isr_interrupt105_entry();
    pub(super) fn hal_i586_isr_interrupt106_entry();
    pub(super) fn hal_i586_isr_interrupt107_entry();
    pub(super) fn hal_i586_isr_interrupt108_entry();
    pub(super) fn hal_i586_isr_interrupt109_entry();
    pub(super) fn hal_i586_isr_interrupt110_entry();
    pub(super) fn hal_i586_isr_interrupt111_entry();
    pub(super) fn hal_i586_isr_interrupt112_entry();
    pub(super) fn hal_i586_isr_interrupt113_entry();
    pub(super) fn hal_i586_isr_interrupt114_entry();
    pub(super) fn hal_i586_isr_interrupt115_entry();
    pub(super) fn hal_i586_isr_interrupt116_entry();
    pub(super) fn hal_i586_isr_interrupt117_entry();
    pub(super) fn hal_i586_isr_interrupt118_entry();
    pub(super) fn hal_i586_isr_interrupt119_entry();
    pub(super) fn hal_i586_isr_interrupt120_entry();
    pub(super) fn hal_i586_isr_interrupt121_entry();
    pub(super) fn hal_i586_isr_interrupt122_entry();
    pub(super) fn hal_i586_isr_interrupt123_entry();
    pub(super) fn hal_i586_isr_interrupt124_entry();
    pub(super) fn hal_i586_isr_interrupt125_entry();
    pub(super) fn hal_i586_isr_interrupt126_entry();
    pub(super) fn hal_i586_isr_interrupt127_entry();
    pub(super) fn hal_i586_isr_interrupt128_entry();
    pub(super) fn hal_i586_isr_interrupt129_entry();
    pub(super) fn hal_i586_isr_interrupt130_entry();
    pub(super) fn hal_i586_isr_interrupt131_entry();
    pub(super) fn hal_i586_isr_interrupt132_entry();
    pub(super) fn hal_i586_isr_interrupt133_entry();
    pub(super) fn hal_i586_isr_interrupt134_entry();
    pub(super) fn hal_i586_isr_interrupt135_entry();
    pub(super) fn hal_i586_isr_interrupt136_entry();
    pub(super) fn hal_i586_isr_interrupt137_entry();
    pub(super) fn hal_i586_isr_interrupt138_entry();
    pub(super) fn hal_i586_isr_interrupt139_entry();
    pub(super) fn hal_i586_isr_interrupt140_entry();
    pub(super) fn hal_i586_isr_interrupt141_entry();
    pub(super) fn hal_i586_isr_interrupt142_entry();
    pub(super) fn hal_i586_isr_interrupt143_entry();
    pub(super) fn hal_i586_isr_interrupt144_entry();
    pub(super) fn hal_i586_isr_interrupt145_entry();
    pub(super) fn hal_i586_isr_interrupt146_entry();
    pub(super) fn hal_i586_isr_interrupt147_entry();
    pub(super) fn hal_i586_isr_interrupt148_entry();
    pub(super) fn hal_i586_isr_interrupt149_entry();
    pub(super) fn hal_i586_isr_interrupt150_entry();
    pub(super) fn hal_i586_isr_interrupt151_entry();
    pub(super) fn hal_i586_isr_interrupt152_entry();
    pub(super) fn hal_i586_isr_interrupt153_entry();
    pub(super) fn hal_i586_isr_interrupt154_entry();
    pub(super) fn hal_i586_isr_interrupt155_entry();
    pub(super) fn hal_i586_isr_interrupt156_entry();
    pub(super) fn hal_i586_isr_interrupt157_entry();
    pub(super) fn hal_i586_isr_interrupt158_entry();
    pub(super) fn hal_i586_isr_interrupt159_entry();
    pub(super) fn hal_i586_isr_interrupt160_entry();
    pub(super) fn hal_i586_isr_interrupt161_entry();
    pub(super) fn hal_i586_isr_interrupt162_entry();
    pub(super) fn hal_i586_isr_interrupt163_entry();
    pub(super) fn hal_i586_isr_interrupt164_entry();
    pub(super) fn hal_i586_isr_interrupt165_entry();
    pub(super) fn hal_i586_isr_interrupt166_entry();
    pub(super) fn hal_i586_isr_interrupt167_entry();
    pub(super) fn hal_i586_isr_interrupt168_entry();
    pub(super) fn hal_i586_isr_interrupt169_entry();
    pub(super) fn hal_i586_isr_interrupt170_entry();
    pub(super) fn hal_i586_isr_interrupt171_entry();
    pub(super) fn hal_i586_isr_interrupt172_entry();
    pub(super) fn hal_i586_isr_interrupt173_entry();
    pub(super) fn hal_i586_isr_interrupt174_entry();
    pub(super) fn hal_i586_isr_interrupt175_entry();
    pub(super) fn hal_i586_isr_interrupt176_entry();
    pub(super) fn hal_i586_isr_interrupt177_entry();
    pub(super) fn hal_i586_isr_interrupt178_entry();
    pub(super) fn hal_i586_isr_interrupt179_entry();
    pub(super) fn hal_i586_isr_interrupt180_entry();
    pub(super) fn hal_i586_isr_interrupt181_entry();
    pub(super) fn hal_i586_isr_interrupt182_entry();
    pub(super) fn hal_i586_isr_interrupt183_entry();
    pub(super) fn hal_i586_isr_interrupt184_entry();
    pub(super) fn hal_i586_isr_interrupt185_entry();
    pub(super) fn hal_i586_isr_interrupt186_entry();
    pub(super) fn hal_i586_isr_interrupt187_entry();
    pub(super) fn hal_i586_isr_interrupt188_entry();
    pub(super) fn hal_i586_isr_interrupt189_entry();
    pub(super) fn hal_i586_isr_interrupt190_entry();
    pub(super) fn hal_i586_isr_interrupt191_entry();
    pub(super) fn hal_i586_isr_interrupt192_entry();
    pub(super) fn hal_i586_isr_interrupt193_entry();
    pub(super) fn hal_i586_isr_interrupt194_entry();
    pub(super) fn hal_i586_isr_interrupt195_entry();
    pub(super) fn hal_i586_isr_interrupt196_entry();
    pub(super) fn hal_i586_isr_interrupt197_entry();
    pub(super) fn hal_i586_isr_interrupt198_entry();
    pub(super) fn hal_i586_isr_interrupt199_entry();
    pub(super) fn hal_i586_isr_interrupt200_entry();
    pub(super) fn hal_i586_isr_interrupt201_entry();
    pub(super) fn hal_i586_isr_interrupt202_entry();
    pub(super) fn hal_i586_isr_interrupt203_entry();
    pub(super) fn hal_i586_isr_interrupt204_entry();
    pub(super) fn hal_i586_isr_interrupt205_entry();
    pub(super) fn hal_i586_isr_interrupt206_entry();
    pub(super) fn hal_i586_isr_interrupt207_entry();
    pub(super) fn hal_i586_isr_interrupt208_entry();
    pub(super) fn hal_i586_isr_interrupt209_entry();
    pub(super) fn hal_i586_isr_interrupt210_entry();
    pub(super) fn hal_i586_isr_interrupt211_entry();
    pub(super) fn hal_i586_isr_interrupt212_entry();
    pub(super) fn hal_i586_isr_interrupt213_entry();
    pub(super) fn hal_i586_isr_interrupt214_entry();
    pub(super) fn hal_i586_isr_interrupt215_entry();
    pub(super) fn hal_i586_isr_interrupt216_entry();
    pub(super) fn hal_i586_isr_interrupt217_entry();
    pub(super) fn hal_i586_isr_interrupt218_entry();
    pub(super) fn hal_i586_isr_interrupt219_entry();
    pub(super) fn hal_i586_isr_interrupt220_entry();
    pub(super) fn hal_i586_isr_interrupt221_entry();
    pub(super) fn hal_i586_isr_interrupt222_entry();
    pub(super) fn hal_i586_isr_interrupt223_entry();
    pub(super) fn hal_i586_isr_interrupt224_entry();
    pub(super) fn hal_i586_isr_interrupt225_entry();
    pub(super) fn hal_i586_isr_interrupt226_entry();
    pub(super) fn hal_i586_isr_interrupt227_entry();
    pub(super) fn hal_i586_isr_interrupt228_entry();
    pub(super) fn hal_i586_isr_interrupt229_entry();
    pub(super) fn hal_i586_isr_interrupt230_entry();
    pub(super) fn hal_i586_isr_interrupt231_entry();
    pub(super) fn hal_i586_isr_interrupt232_entry();
    pub(super) fn hal_i586_isr_interrupt233_entry();
    pub(super) fn hal_i586_isr_interrupt234_entry();
    pub(super) fn hal_i586_isr_interrupt235_entry();
    pub(super) fn hal_i586_isr_interrupt236_entry();
    pub(super) fn hal_i586_isr_interrupt237_entry();
    pub(super) fn hal_i586_isr_interrupt238_entry();
    pub(super) fn hal_i586_isr_interrupt239_entry();
    pub(super) fn hal_i586_isr_interrupt240_entry();
    pub(super) fn hal_i586_isr_interrupt241_entry();
    pub(super) fn hal_i586_isr_interrupt242_entry();
    pub(super) fn hal_i586_isr_interrupt243_entry();
    pub(super) fn hal_i586_isr_interrupt244_entry();
    pub(super) fn hal_i586_isr_interrupt245_entry();
    pub(super) fn hal_i586_isr_interrupt246_entry();
    pub(super) fn hal_i586_isr_interrupt247_entry();
    pub(super) fn hal_i586_isr_interrupt248_entry();
    pub(super) fn hal_i586_isr_interrupt249_entry();
    pub(super) fn hal_i586_isr_interrupt250_entry();
    pub(super) fn hal_i586_isr_interrupt251_entry();
    pub(super) fn hal_i586_isr_interrupt252_entry();
    pub(super) fn hal_i586_isr_interrupt253_entry();
    pub(super) fn hal_i586_isr_interrupt254_entry();
    pub(super) fn hal_i586_isr_interrupt255_entry();
}

global_asm!(
    r#"
.section .text

.global hal_i586_isr_exception0_entry,  hal_i586_isr_exception1_entry,  hal_i586_isr_exception2_entry,  hal_i586_isr_exception3_entry
.global hal_i586_isr_exception4_entry,  hal_i586_isr_exception5_entry,  hal_i586_isr_exception6_entry,  hal_i586_isr_exception7_entry
.global hal_i586_isr_exception8_entry,  hal_i586_isr_exception9_entry,  hal_i586_isr_exception10_entry, hal_i586_isr_exception11_entry
.global hal_i586_isr_exception12_entry, hal_i586_isr_exception13_entry, hal_i586_isr_exception14_entry, hal_i586_isr_exception15_entry
.global hal_i586_isr_exception16_entry, hal_i586_isr_exception17_entry, hal_i586_isr_exception18_entry, hal_i586_isr_exception19_entry
.global hal_i586_isr_exception20_entry, hal_i586_isr_exception21_entry, hal_i586_isr_exception22_entry, hal_i586_isr_exception23_entry
.global hal_i586_isr_exception24_entry, hal_i586_isr_exception25_entry, hal_i586_isr_exception26_entry, hal_i586_isr_exception27_entry
.global hal_i586_isr_exception28_entry, hal_i586_isr_exception29_entry, hal_i586_isr_exception30_entry, hal_i586_isr_exception31_entry

.global hal_i586_isr_interrupt32_entry,  hal_i586_isr_interrupt33_entry,  hal_i586_isr_interrupt34_entry,  hal_i586_isr_interrupt35_entry
.global hal_i586_isr_interrupt36_entry,  hal_i586_isr_interrupt37_entry,  hal_i586_isr_interrupt38_entry,  hal_i586_isr_interrupt39_entry
.global hal_i586_isr_interrupt40_entry,  hal_i586_isr_interrupt41_entry,  hal_i586_isr_interrupt42_entry,  hal_i586_isr_interrupt43_entry
.global hal_i586_isr_interrupt44_entry,  hal_i586_isr_interrupt45_entry,  hal_i586_isr_interrupt46_entry,  hal_i586_isr_interrupt47_entry
.global hal_i586_isr_interrupt48_entry,  hal_i586_isr_interrupt49_entry,  hal_i586_isr_interrupt50_entry,  hal_i586_isr_interrupt51_entry
.global hal_i586_isr_interrupt52_entry,  hal_i586_isr_interrupt53_entry,  hal_i586_isr_interrupt54_entry,  hal_i586_isr_interrupt55_entry
.global hal_i586_isr_interrupt56_entry,  hal_i586_isr_interrupt57_entry,  hal_i586_isr_interrupt58_entry,  hal_i586_isr_interrupt59_entry
.global hal_i586_isr_interrupt60_entry,  hal_i586_isr_interrupt61_entry,  hal_i586_isr_interrupt62_entry,  hal_i586_isr_interrupt63_entry
.global hal_i586_isr_interrupt64_entry,  hal_i586_isr_interrupt65_entry,  hal_i586_isr_interrupt66_entry,  hal_i586_isr_interrupt67_entry
.global hal_i586_isr_interrupt68_entry,  hal_i586_isr_interrupt69_entry,  hal_i586_isr_interrupt70_entry,  hal_i586_isr_interrupt71_entry
.global hal_i586_isr_interrupt72_entry,  hal_i586_isr_interrupt73_entry,  hal_i586_isr_interrupt74_entry,  hal_i586_isr_interrupt75_entry
.global hal_i586_isr_interrupt76_entry,  hal_i586_isr_interrupt77_entry,  hal_i586_isr_interrupt78_entry,  hal_i586_isr_interrupt79_entry
.global hal_i586_isr_interrupt80_entry,  hal_i586_isr_interrupt81_entry,  hal_i586_isr_interrupt82_entry,  hal_i586_isr_interrupt83_entry
.global hal_i586_isr_interrupt84_entry,  hal_i586_isr_interrupt85_entry,  hal_i586_isr_interrupt86_entry,  hal_i586_isr_interrupt87_entry
.global hal_i586_isr_interrupt88_entry,  hal_i586_isr_interrupt89_entry,  hal_i586_isr_interrupt90_entry,  hal_i586_isr_interrupt91_entry
.global hal_i586_isr_interrupt92_entry,  hal_i586_isr_interrupt93_entry,  hal_i586_isr_interrupt94_entry,  hal_i586_isr_interrupt95_entry
.global hal_i586_isr_interrupt96_entry,  hal_i586_isr_interrupt97_entry,  hal_i586_isr_interrupt98_entry,  hal_i586_isr_interrupt99_entry
.global hal_i586_isr_interrupt100_entry, hal_i586_isr_interrupt101_entry, hal_i586_isr_interrupt102_entry, hal_i586_isr_interrupt103_entry
.global hal_i586_isr_interrupt104_entry, hal_i586_isr_interrupt105_entry, hal_i586_isr_interrupt106_entry, hal_i586_isr_interrupt107_entry
.global hal_i586_isr_interrupt108_entry, hal_i586_isr_interrupt109_entry, hal_i586_isr_interrupt110_entry, hal_i586_isr_interrupt111_entry
.global hal_i586_isr_interrupt112_entry, hal_i586_isr_interrupt113_entry, hal_i586_isr_interrupt114_entry, hal_i586_isr_interrupt115_entry
.global hal_i586_isr_interrupt116_entry, hal_i586_isr_interrupt117_entry, hal_i586_isr_interrupt118_entry, hal_i586_isr_interrupt119_entry
.global hal_i586_isr_interrupt120_entry, hal_i586_isr_interrupt121_entry, hal_i586_isr_interrupt122_entry, hal_i586_isr_interrupt123_entry
.global hal_i586_isr_interrupt124_entry, hal_i586_isr_interrupt125_entry, hal_i586_isr_interrupt126_entry, hal_i586_isr_interrupt127_entry
.global hal_i586_isr_interrupt128_entry, hal_i586_isr_interrupt129_entry, hal_i586_isr_interrupt130_entry, hal_i586_isr_interrupt131_entry
.global hal_i586_isr_interrupt132_entry, hal_i586_isr_interrupt133_entry, hal_i586_isr_interrupt134_entry, hal_i586_isr_interrupt135_entry
.global hal_i586_isr_interrupt136_entry, hal_i586_isr_interrupt137_entry, hal_i586_isr_interrupt138_entry, hal_i586_isr_interrupt139_entry
.global hal_i586_isr_interrupt140_entry, hal_i586_isr_interrupt141_entry, hal_i586_isr_interrupt142_entry, hal_i586_isr_interrupt143_entry
.global hal_i586_isr_interrupt144_entry, hal_i586_isr_interrupt145_entry, hal_i586_isr_interrupt146_entry, hal_i586_isr_interrupt147_entry
.global hal_i586_isr_interrupt148_entry, hal_i586_isr_interrupt149_entry, hal_i586_isr_interrupt150_entry, hal_i586_isr_interrupt151_entry
.global hal_i586_isr_interrupt152_entry, hal_i586_isr_interrupt153_entry, hal_i586_isr_interrupt154_entry, hal_i586_isr_interrupt155_entry
.global hal_i586_isr_interrupt156_entry, hal_i586_isr_interrupt157_entry, hal_i586_isr_interrupt158_entry, hal_i586_isr_interrupt159_entry
.global hal_i586_isr_interrupt160_entry, hal_i586_isr_interrupt161_entry, hal_i586_isr_interrupt162_entry, hal_i586_isr_interrupt163_entry
.global hal_i586_isr_interrupt164_entry, hal_i586_isr_interrupt165_entry, hal_i586_isr_interrupt166_entry, hal_i586_isr_interrupt167_entry
.global hal_i586_isr_interrupt168_entry, hal_i586_isr_interrupt169_entry, hal_i586_isr_interrupt170_entry, hal_i586_isr_interrupt171_entry
.global hal_i586_isr_interrupt172_entry, hal_i586_isr_interrupt173_entry, hal_i586_isr_interrupt174_entry, hal_i586_isr_interrupt175_entry
.global hal_i586_isr_interrupt176_entry, hal_i586_isr_interrupt177_entry, hal_i586_isr_interrupt178_entry, hal_i586_isr_interrupt179_entry
.global hal_i586_isr_interrupt180_entry, hal_i586_isr_interrupt181_entry, hal_i586_isr_interrupt182_entry, hal_i586_isr_interrupt183_entry
.global hal_i586_isr_interrupt184_entry, hal_i586_isr_interrupt185_entry, hal_i586_isr_interrupt186_entry, hal_i586_isr_interrupt187_entry
.global hal_i586_isr_interrupt188_entry, hal_i586_isr_interrupt189_entry, hal_i586_isr_interrupt190_entry, hal_i586_isr_interrupt191_entry
.global hal_i586_isr_interrupt192_entry, hal_i586_isr_interrupt193_entry, hal_i586_isr_interrupt194_entry, hal_i586_isr_interrupt195_entry
.global hal_i586_isr_interrupt196_entry, hal_i586_isr_interrupt197_entry, hal_i586_isr_interrupt198_entry, hal_i586_isr_interrupt199_entry
.global hal_i586_isr_interrupt200_entry, hal_i586_isr_interrupt201_entry, hal_i586_isr_interrupt202_entry, hal_i586_isr_interrupt203_entry
.global hal_i586_isr_interrupt204_entry, hal_i586_isr_interrupt205_entry, hal_i586_isr_interrupt206_entry, hal_i586_isr_interrupt207_entry
.global hal_i586_isr_interrupt208_entry, hal_i586_isr_interrupt209_entry, hal_i586_isr_interrupt210_entry, hal_i586_isr_interrupt211_entry
.global hal_i586_isr_interrupt212_entry, hal_i586_isr_interrupt213_entry, hal_i586_isr_interrupt214_entry, hal_i586_isr_interrupt215_entry
.global hal_i586_isr_interrupt216_entry, hal_i586_isr_interrupt217_entry, hal_i586_isr_interrupt218_entry, hal_i586_isr_interrupt219_entry
.global hal_i586_isr_interrupt220_entry, hal_i586_isr_interrupt221_entry, hal_i586_isr_interrupt222_entry, hal_i586_isr_interrupt223_entry
.global hal_i586_isr_interrupt224_entry, hal_i586_isr_interrupt225_entry, hal_i586_isr_interrupt226_entry, hal_i586_isr_interrupt227_entry
.global hal_i586_isr_interrupt228_entry, hal_i586_isr_interrupt229_entry, hal_i586_isr_interrupt230_entry, hal_i586_isr_interrupt231_entry
.global hal_i586_isr_interrupt232_entry, hal_i586_isr_interrupt233_entry, hal_i586_isr_interrupt234_entry, hal_i586_isr_interrupt235_entry
.global hal_i586_isr_interrupt236_entry, hal_i586_isr_interrupt237_entry, hal_i586_isr_interrupt238_entry, hal_i586_isr_interrupt239_entry
.global hal_i586_isr_interrupt240_entry, hal_i586_isr_interrupt241_entry, hal_i586_isr_interrupt242_entry, hal_i586_isr_interrupt243_entry
.global hal_i586_isr_interrupt244_entry, hal_i586_isr_interrupt245_entry, hal_i586_isr_interrupt246_entry, hal_i586_isr_interrupt247_entry
.global hal_i586_isr_interrupt248_entry, hal_i586_isr_interrupt249_entry, hal_i586_isr_interrupt250_entry, hal_i586_isr_interrupt251_entry
.global hal_i586_isr_interrupt252_entry, hal_i586_isr_interrupt253_entry, hal_i586_isr_interrupt254_entry, hal_i586_isr_interrupt255_entry

.macro isr_exc_header
    cli
.endm

.macro push_fake_code
    push 0
.endm

.macro save_regs
    pusha
    mov eax, ds
    push eax
    mov eax, es
    push eax
    mov eax, fs
    push eax
    mov eax, gs
    push eax
.endm

.macro restore_regs
    add esp, 8 /* Skip FS and GS */
    pop eax
    mov es, eax
    pop eax
    mov ds, eax
    popa
.endm

.macro isr_footer
    add esp, 4
    iretd
.endm

.macro call_handler n
    mov ebx, esp
    /* Align the stack */
    and esp, ~0x0F
    /* Call the handler */
    push ebx /* Arg 1 */
    push \n  /* Arg 0 */
    call trap_manager_trap
    /* Restore ESP */
    mov esp, ebx
.endm

.macro isr_excentry n
    isr_exc_header
    save_regs
    call_handler \n
    restore_regs
    isr_footer 
.endm

.macro isr_excentry_nocode n
    isr_exc_header
    push_fake_code
    save_regs
    call_handler \n
    restore_regs
    isr_footer 
.endm

.macro isr_intentry n
    push_fake_code
    save_regs
    call_handler \n
    restore_regs
    isr_footer 
.endm

hal_i586_isr_exception0_entry:  isr_excentry_nocode 0
hal_i586_isr_exception1_entry:  isr_excentry_nocode 1
hal_i586_isr_exception2_entry:  isr_excentry_nocode 2
hal_i586_isr_exception3_entry:  isr_excentry_nocode 3
hal_i586_isr_exception4_entry:  isr_excentry_nocode 4
hal_i586_isr_exception5_entry:  isr_excentry_nocode 5
hal_i586_isr_exception6_entry:  isr_excentry_nocode 6
/* hal_i586_isr_exception7_entry:  isr_excentry_nocode 7 */
hal_i586_isr_exception7_entry:  jmp hal_i586_isr_exception7_entry
hal_i586_isr_exception8_entry:  isr_excentry        8
hal_i586_isr_exception9_entry:  isr_excentry_nocode 9
hal_i586_isr_exception10_entry: isr_excentry        10
hal_i586_isr_exception11_entry: isr_excentry        11
hal_i586_isr_exception12_entry: isr_excentry        12
hal_i586_isr_exception13_entry: isr_excentry        13
hal_i586_isr_exception14_entry: isr_excentry        14
hal_i586_isr_exception15_entry: isr_excentry_nocode 15
hal_i586_isr_exception16_entry: isr_excentry_nocode 16
hal_i586_isr_exception17_entry: isr_excentry        17
hal_i586_isr_exception18_entry: isr_excentry_nocode 18
hal_i586_isr_exception19_entry: isr_excentry_nocode 19
hal_i586_isr_exception20_entry: isr_excentry_nocode 20
hal_i586_isr_exception21_entry: isr_excentry        21
hal_i586_isr_exception22_entry: isr_excentry_nocode 22
hal_i586_isr_exception23_entry: isr_excentry_nocode 23
hal_i586_isr_exception24_entry: isr_excentry_nocode 24
hal_i586_isr_exception25_entry: isr_excentry_nocode 25
hal_i586_isr_exception26_entry: isr_excentry_nocode 26
hal_i586_isr_exception27_entry: isr_excentry_nocode 27
hal_i586_isr_exception28_entry: isr_excentry_nocode 28
hal_i586_isr_exception29_entry: isr_excentry        29
hal_i586_isr_exception30_entry: isr_excentry        30
hal_i586_isr_exception31_entry: isr_excentry_nocode 31

hal_i586_isr_interrupt32_entry:  isr_intentry 32
hal_i586_isr_interrupt33_entry:  isr_intentry 33
hal_i586_isr_interrupt34_entry:  isr_intentry 34
hal_i586_isr_interrupt35_entry:  isr_intentry 35
hal_i586_isr_interrupt36_entry:  isr_intentry 36
hal_i586_isr_interrupt37_entry:  isr_intentry 37
hal_i586_isr_interrupt38_entry:  isr_intentry 38
hal_i586_isr_interrupt39_entry:  isr_intentry 39
hal_i586_isr_interrupt40_entry:  isr_intentry 40
hal_i586_isr_interrupt41_entry:  isr_intentry 41
hal_i586_isr_interrupt42_entry:  isr_intentry 42
hal_i586_isr_interrupt43_entry:  isr_intentry 43
hal_i586_isr_interrupt44_entry:  isr_intentry 44
hal_i586_isr_interrupt45_entry:  isr_intentry 45
hal_i586_isr_interrupt46_entry:  isr_intentry 46
hal_i586_isr_interrupt47_entry:  isr_intentry 47
hal_i586_isr_interrupt48_entry:  isr_intentry 48
hal_i586_isr_interrupt49_entry:  isr_intentry 49
hal_i586_isr_interrupt50_entry:  isr_intentry 50
hal_i586_isr_interrupt51_entry:  isr_intentry 51
hal_i586_isr_interrupt52_entry:  isr_intentry 52
hal_i586_isr_interrupt53_entry:  isr_intentry 53
hal_i586_isr_interrupt54_entry:  isr_intentry 54
hal_i586_isr_interrupt55_entry:  isr_intentry 55
hal_i586_isr_interrupt56_entry:  isr_intentry 56
hal_i586_isr_interrupt57_entry:  isr_intentry 57
hal_i586_isr_interrupt58_entry:  isr_intentry 58
hal_i586_isr_interrupt59_entry:  isr_intentry 59
hal_i586_isr_interrupt60_entry:  isr_intentry 60
hal_i586_isr_interrupt61_entry:  isr_intentry 61
hal_i586_isr_interrupt62_entry:  isr_intentry 62
hal_i586_isr_interrupt63_entry:  isr_intentry 63
hal_i586_isr_interrupt64_entry:  isr_intentry 64
hal_i586_isr_interrupt65_entry:  isr_intentry 65
hal_i586_isr_interrupt66_entry:  isr_intentry 66
hal_i586_isr_interrupt67_entry:  isr_intentry 67
hal_i586_isr_interrupt68_entry:  isr_intentry 68
hal_i586_isr_interrupt69_entry:  isr_intentry 69
hal_i586_isr_interrupt70_entry:  isr_intentry 70
hal_i586_isr_interrupt71_entry:  isr_intentry 71
hal_i586_isr_interrupt72_entry:  isr_intentry 72
hal_i586_isr_interrupt73_entry:  isr_intentry 73
hal_i586_isr_interrupt74_entry:  isr_intentry 74
hal_i586_isr_interrupt75_entry:  isr_intentry 75
hal_i586_isr_interrupt76_entry:  isr_intentry 76
hal_i586_isr_interrupt77_entry:  isr_intentry 77
hal_i586_isr_interrupt78_entry:  isr_intentry 78
hal_i586_isr_interrupt79_entry:  isr_intentry 79
hal_i586_isr_interrupt80_entry:  isr_intentry 80
hal_i586_isr_interrupt81_entry:  isr_intentry 81
hal_i586_isr_interrupt82_entry:  isr_intentry 82
hal_i586_isr_interrupt83_entry:  isr_intentry 83
hal_i586_isr_interrupt84_entry:  isr_intentry 84
hal_i586_isr_interrupt85_entry:  isr_intentry 85
hal_i586_isr_interrupt86_entry:  isr_intentry 86
hal_i586_isr_interrupt87_entry:  isr_intentry 87
hal_i586_isr_interrupt88_entry:  isr_intentry 88
hal_i586_isr_interrupt89_entry:  isr_intentry 89
hal_i586_isr_interrupt90_entry:  isr_intentry 90
hal_i586_isr_interrupt91_entry:  isr_intentry 91
hal_i586_isr_interrupt92_entry:  isr_intentry 92
hal_i586_isr_interrupt93_entry:  isr_intentry 93
hal_i586_isr_interrupt94_entry:  isr_intentry 94
hal_i586_isr_interrupt95_entry:  isr_intentry 95
hal_i586_isr_interrupt96_entry:  isr_intentry 96
hal_i586_isr_interrupt97_entry:  isr_intentry 97
hal_i586_isr_interrupt98_entry:  isr_intentry 98
hal_i586_isr_interrupt99_entry:  isr_intentry 99
hal_i586_isr_interrupt100_entry: isr_intentry 100
hal_i586_isr_interrupt101_entry: isr_intentry 101
hal_i586_isr_interrupt102_entry: isr_intentry 102
hal_i586_isr_interrupt103_entry: isr_intentry 103
hal_i586_isr_interrupt104_entry: isr_intentry 104
hal_i586_isr_interrupt105_entry: isr_intentry 105
hal_i586_isr_interrupt106_entry: isr_intentry 106
hal_i586_isr_interrupt107_entry: isr_intentry 107
hal_i586_isr_interrupt108_entry: isr_intentry 108
hal_i586_isr_interrupt109_entry: isr_intentry 109
hal_i586_isr_interrupt110_entry: isr_intentry 110
hal_i586_isr_interrupt111_entry: isr_intentry 111
hal_i586_isr_interrupt112_entry: isr_intentry 112
hal_i586_isr_interrupt113_entry: isr_intentry 113
hal_i586_isr_interrupt114_entry: isr_intentry 114
hal_i586_isr_interrupt115_entry: isr_intentry 115
hal_i586_isr_interrupt116_entry: isr_intentry 116
hal_i586_isr_interrupt117_entry: isr_intentry 117
hal_i586_isr_interrupt118_entry: isr_intentry 118
hal_i586_isr_interrupt119_entry: isr_intentry 119
hal_i586_isr_interrupt120_entry: isr_intentry 120
hal_i586_isr_interrupt121_entry: isr_intentry 121
hal_i586_isr_interrupt122_entry: isr_intentry 122
hal_i586_isr_interrupt123_entry: isr_intentry 123
hal_i586_isr_interrupt124_entry: isr_intentry 124
hal_i586_isr_interrupt125_entry: isr_intentry 125
hal_i586_isr_interrupt126_entry: isr_intentry 126
hal_i586_isr_interrupt127_entry: isr_intentry 127
hal_i586_isr_interrupt128_entry: isr_intentry 128
hal_i586_isr_interrupt129_entry: isr_intentry 129
hal_i586_isr_interrupt130_entry: isr_intentry 130
hal_i586_isr_interrupt131_entry: isr_intentry 131
hal_i586_isr_interrupt132_entry: isr_intentry 132
hal_i586_isr_interrupt133_entry: isr_intentry 133
hal_i586_isr_interrupt134_entry: isr_intentry 134
hal_i586_isr_interrupt135_entry: isr_intentry 135
hal_i586_isr_interrupt136_entry: isr_intentry 136
hal_i586_isr_interrupt137_entry: isr_intentry 137
hal_i586_isr_interrupt138_entry: isr_intentry 138
hal_i586_isr_interrupt139_entry: isr_intentry 139
hal_i586_isr_interrupt140_entry: isr_intentry 140
hal_i586_isr_interrupt141_entry: isr_intentry 141
hal_i586_isr_interrupt142_entry: isr_intentry 142
hal_i586_isr_interrupt143_entry: isr_intentry 143
hal_i586_isr_interrupt144_entry: isr_intentry 144
hal_i586_isr_interrupt145_entry: isr_intentry 145
hal_i586_isr_interrupt146_entry: isr_intentry 146
hal_i586_isr_interrupt147_entry: isr_intentry 147
hal_i586_isr_interrupt148_entry: isr_intentry 148
hal_i586_isr_interrupt149_entry: isr_intentry 149
hal_i586_isr_interrupt150_entry: isr_intentry 150
hal_i586_isr_interrupt151_entry: isr_intentry 151
hal_i586_isr_interrupt152_entry: isr_intentry 152
hal_i586_isr_interrupt153_entry: isr_intentry 153
hal_i586_isr_interrupt154_entry: isr_intentry 154
hal_i586_isr_interrupt155_entry: isr_intentry 155
hal_i586_isr_interrupt156_entry: isr_intentry 156
hal_i586_isr_interrupt157_entry: isr_intentry 157
hal_i586_isr_interrupt158_entry: isr_intentry 158
hal_i586_isr_interrupt159_entry: isr_intentry 159
hal_i586_isr_interrupt160_entry: isr_intentry 160
hal_i586_isr_interrupt161_entry: isr_intentry 161
hal_i586_isr_interrupt162_entry: isr_intentry 162
hal_i586_isr_interrupt163_entry: isr_intentry 163
hal_i586_isr_interrupt164_entry: isr_intentry 164
hal_i586_isr_interrupt165_entry: isr_intentry 165
hal_i586_isr_interrupt166_entry: isr_intentry 166
hal_i586_isr_interrupt167_entry: isr_intentry 167
hal_i586_isr_interrupt168_entry: isr_intentry 168
hal_i586_isr_interrupt169_entry: isr_intentry 169
hal_i586_isr_interrupt170_entry: isr_intentry 170
hal_i586_isr_interrupt171_entry: isr_intentry 171
hal_i586_isr_interrupt172_entry: isr_intentry 172
hal_i586_isr_interrupt173_entry: isr_intentry 173
hal_i586_isr_interrupt174_entry: isr_intentry 174
hal_i586_isr_interrupt175_entry: isr_intentry 175
hal_i586_isr_interrupt176_entry: isr_intentry 176
hal_i586_isr_interrupt177_entry: isr_intentry 177
hal_i586_isr_interrupt178_entry: isr_intentry 178
hal_i586_isr_interrupt179_entry: isr_intentry 179
hal_i586_isr_interrupt180_entry: isr_intentry 180
hal_i586_isr_interrupt181_entry: isr_intentry 181
hal_i586_isr_interrupt182_entry: isr_intentry 182
hal_i586_isr_interrupt183_entry: isr_intentry 183
hal_i586_isr_interrupt184_entry: isr_intentry 184
hal_i586_isr_interrupt185_entry: isr_intentry 185
hal_i586_isr_interrupt186_entry: isr_intentry 186
hal_i586_isr_interrupt187_entry: isr_intentry 187
hal_i586_isr_interrupt188_entry: isr_intentry 188
hal_i586_isr_interrupt189_entry: isr_intentry 189
hal_i586_isr_interrupt190_entry: isr_intentry 190
hal_i586_isr_interrupt191_entry: isr_intentry 191
hal_i586_isr_interrupt192_entry: isr_intentry 192
hal_i586_isr_interrupt193_entry: isr_intentry 193
hal_i586_isr_interrupt194_entry: isr_intentry 194
hal_i586_isr_interrupt195_entry: isr_intentry 195
hal_i586_isr_interrupt196_entry: isr_intentry 196
hal_i586_isr_interrupt197_entry: isr_intentry 197
hal_i586_isr_interrupt198_entry: isr_intentry 198
hal_i586_isr_interrupt199_entry: isr_intentry 199
hal_i586_isr_interrupt200_entry: isr_intentry 200
hal_i586_isr_interrupt201_entry: isr_intentry 201
hal_i586_isr_interrupt202_entry: isr_intentry 202
hal_i586_isr_interrupt203_entry: isr_intentry 203
hal_i586_isr_interrupt204_entry: isr_intentry 204
hal_i586_isr_interrupt205_entry: isr_intentry 205
hal_i586_isr_interrupt206_entry: isr_intentry 206
hal_i586_isr_interrupt207_entry: isr_intentry 207
hal_i586_isr_interrupt208_entry: isr_intentry 208
hal_i586_isr_interrupt209_entry: isr_intentry 209
hal_i586_isr_interrupt210_entry: isr_intentry 210
hal_i586_isr_interrupt211_entry: isr_intentry 211
hal_i586_isr_interrupt212_entry: isr_intentry 212
hal_i586_isr_interrupt213_entry: isr_intentry 213
hal_i586_isr_interrupt214_entry: isr_intentry 214
hal_i586_isr_interrupt215_entry: isr_intentry 215
hal_i586_isr_interrupt216_entry: isr_intentry 216
hal_i586_isr_interrupt217_entry: isr_intentry 217
hal_i586_isr_interrupt218_entry: isr_intentry 218
hal_i586_isr_interrupt219_entry: isr_intentry 219
hal_i586_isr_interrupt220_entry: isr_intentry 220
hal_i586_isr_interrupt221_entry: isr_intentry 221
hal_i586_isr_interrupt222_entry: isr_intentry 222
hal_i586_isr_interrupt223_entry: isr_intentry 223
hal_i586_isr_interrupt224_entry: isr_intentry 224
hal_i586_isr_interrupt225_entry: isr_intentry 225
hal_i586_isr_interrupt226_entry: isr_intentry 226
hal_i586_isr_interrupt227_entry: isr_intentry 227
hal_i586_isr_interrupt228_entry: isr_intentry 228
hal_i586_isr_interrupt229_entry: isr_intentry 229
hal_i586_isr_interrupt230_entry: isr_intentry 230
hal_i586_isr_interrupt231_entry: isr_intentry 231
hal_i586_isr_interrupt232_entry: isr_intentry 232
hal_i586_isr_interrupt233_entry: isr_intentry 233
hal_i586_isr_interrupt234_entry: isr_intentry 234
hal_i586_isr_interrupt235_entry: isr_intentry 235
hal_i586_isr_interrupt236_entry: isr_intentry 236
hal_i586_isr_interrupt237_entry: isr_intentry 237
hal_i586_isr_interrupt238_entry: isr_intentry 238
hal_i586_isr_interrupt239_entry: isr_intentry 239
hal_i586_isr_interrupt240_entry: isr_intentry 240
hal_i586_isr_interrupt241_entry: isr_intentry 241
hal_i586_isr_interrupt242_entry: isr_intentry 242
hal_i586_isr_interrupt243_entry: isr_intentry 243
hal_i586_isr_interrupt244_entry: isr_intentry 244
hal_i586_isr_interrupt245_entry: isr_intentry 245
hal_i586_isr_interrupt246_entry: isr_intentry 246
hal_i586_isr_interrupt247_entry: isr_intentry 247
hal_i586_isr_interrupt248_entry: isr_intentry 248
hal_i586_isr_interrupt249_entry: isr_intentry 249
hal_i586_isr_interrupt250_entry: isr_intentry 250
hal_i586_isr_interrupt251_entry: isr_intentry 251
hal_i586_isr_interrupt252_entry: isr_intentry 252
hal_i586_isr_interrupt253_entry: isr_intentry 253
hal_i586_isr_interrupt254_entry: isr_intentry 254
hal_i586_isr_interrupt255_entry: isr_intentry 255
"#,
);
