library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_shift_rotate_dut is
  port (
    gl_p0_unsigned_input : in unsigned(7 downto 0);
    gl_p1_signed_input : in signed(7 downto 0);
    gl_p2_left_value : out unsigned(7 downto 0);
    gl_p3_logical_right : out unsigned(7 downto 0);
    gl_p4_arithmetic_right : out signed(7 downto 0);
    gl_p5_rotated_left : out unsigned(7 downto 0);
    gl_p6_rotated_right : out unsigned(7 downto 0)
  );
end entity gl_m0_shift_rotate_dut;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_shift_rotate_dut is
  signal gl_s2_left_value : unsigned(7 downto 0);
  signal gl_s3_logical_right : unsigned(7 downto 0);
  signal gl_s4_arithmetic_right : signed(7 downto 0);
  signal gl_s5_rotated_left : unsigned(7 downto 0);
  signal gl_s6_rotated_right : unsigned(7 downto 0);
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_comb_0 : process(all)
  begin
    gl_s2_left_value <= shift_left(gl_p0_unsigned_input, 2);
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s3_logical_right <= shift_right(gl_p0_unsigned_input, 1);
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s4_arithmetic_right <= shift_right(gl_p1_signed_input, 1);
  end process gl_comb_2;
  gl_comb_3 : process(all)
  begin
    gl_s5_rotated_left <= rotate_left(gl_p0_unsigned_input, 1);
  end process gl_comb_3;
  gl_comb_4 : process(all)
  begin
    gl_s6_rotated_right <= rotate_right(gl_p0_unsigned_input, 1);
  end process gl_comb_4;
  gl_p2_left_value <= gl_s2_left_value;
  gl_p3_logical_right <= gl_s3_logical_right;
  gl_p4_arithmetic_right <= gl_s4_arithmetic_right;
  gl_p5_rotated_left <= gl_s5_rotated_left;
  gl_p6_rotated_right <= gl_s6_rotated_right;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_shift_rotate_test is
end entity gl_tb0_shift_rotate_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_shift_rotate_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_unsigned_input : unsigned(7 downto 0) := (others => '0');
  signal gl_tb_s1_signed_input : signed(7 downto 0) := (others => '0');
  signal gl_tb_s2_left_value : unsigned(7 downto 0);
  signal gl_tb_s3_logical_right : unsigned(7 downto 0);
  signal gl_tb_s4_arithmetic_right : signed(7 downto 0);
  signal gl_tb_s5_rotated_left : unsigned(7 downto 0);
  signal gl_tb_s6_rotated_right : unsigned(7 downto 0);
begin
  gl_dut : entity work.gl_m0_shift_rotate_dut
    port map (
      gl_p0_unsigned_input => gl_tb_s0_unsigned_input,
      gl_p1_signed_input => gl_tb_s1_signed_input,
      gl_p2_left_value => gl_tb_s2_left_value,
      gl_p3_logical_right => gl_tb_s3_logical_right,
      gl_p4_arithmetic_right => gl_tb_s4_arithmetic_right,
      gl_p5_rotated_left => gl_tb_s5_rotated_left,
      gl_p6_rotated_right => gl_tb_s6_rotated_right
    );
  gl_stimulus : process
  begin
    gl_tb_s0_unsigned_input <= resize(unsigned'(x"0000000000000081"), 8);
    gl_tb_s1_signed_input <= resize(signed'(x"ffffffffffffff80"), 8);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s2_left_value = resize(unsigned'(x"0000000000000004"), 8))) = '1')
      report "left shift"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s3_logical_right = resize(unsigned'(x"0000000000000040"), 8))) = '1')
      report "logical right"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s4_arithmetic_right = resize(signed'(x"ffffffffffffffc0"), 8))) = '1')
      report "arithmetic right"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s5_rotated_left = resize(unsigned'(x"0000000000000003"), 8))) = '1')
      report "rotate left"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s6_rotated_right = resize(unsigned'(x"00000000000000c0"), 8))) = '1')
      report "rotate right"
      severity error;
    report "GateLisp testbench passed: shift-rotate-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
