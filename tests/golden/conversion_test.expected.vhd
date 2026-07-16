library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_conversion_dut is
  port (
    gl_p0_unsigned_input : in unsigned(7 downto 0);
    gl_p1_signed_input : in signed(15 downto 0);
    gl_p2_extended : out unsigned(15 downto 0);
    gl_p3_truncated : out signed(7 downto 0);
    gl_p4_reinterpreted : out signed(7 downto 0)
  );
end entity gl_m0_conversion_dut;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_conversion_dut is
  signal gl_s2_extended : unsigned(15 downto 0);
  signal gl_s3_truncated : signed(7 downto 0);
  signal gl_s4_reinterpreted : signed(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  function gl_truncate_signed(value : signed; size : positive) return signed is
  begin
    return value(value'low + size - 1 downto value'low);
  end function gl_truncate_signed;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_extended <= resize(gl_p0_unsigned_input, 16);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s3_truncated <= gl_truncate_signed(gl_p1_signed_input, 8);
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s4_reinterpreted <= signed(gl_p0_unsigned_input);
  end process gl_comb_2;
  gl_p2_extended <= gl_s2_extended;
  gl_p3_truncated <= gl_s3_truncated;
  gl_p4_reinterpreted <= gl_s4_reinterpreted;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_conversion_test is
end entity gl_tb0_conversion_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_conversion_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_unsigned_input : unsigned(7 downto 0) := (others => '0');
  signal gl_tb_s1_signed_input : signed(15 downto 0) := (others => '0');
  signal gl_tb_s2_extended : unsigned(15 downto 0);
  signal gl_tb_s3_truncated : signed(7 downto 0);
  signal gl_tb_s4_reinterpreted : signed(7 downto 0);
begin
  gl_dut : entity work.gl_m0_conversion_dut
    port map (
      gl_p0_unsigned_input => gl_tb_s0_unsigned_input,
      gl_p1_signed_input => gl_tb_s1_signed_input,
      gl_p2_extended => gl_tb_s2_extended,
      gl_p3_truncated => gl_tb_s3_truncated,
      gl_p4_reinterpreted => gl_tb_s4_reinterpreted
    );
  gl_stimulus : process
  begin
    gl_tb_s0_unsigned_input <= resize(unsigned'(x"00000000000000ff"), 8);
    gl_tb_s1_signed_input <= resize(signed'(x"00000000000000ff"), 16);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s2_extended = resize(unsigned'(x"00000000000000ff"), 16))) = '1')
      report "unsigned resize must zero extend"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s3_truncated = resize(signed'(x"ffffffffffffffff"), 8))) = '1')
      report "signed truncate must keep low 8 bits"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s4_reinterpreted = resize(signed'(x"ffffffffffffffff"), 8))) = '1')
      report "as-signed must preserve bits"
      severity error;
    report "GateLisp testbench passed: conversion-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
