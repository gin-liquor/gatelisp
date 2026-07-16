library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_split_join is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_input : in unsigned((gl_g0 + gl_g0) - 1 downto 0);
    gl_p1_low : out unsigned(gl_g0 - 1 downto 0);
    gl_p2_high : out unsigned(gl_g0 - 1 downto 0);
    gl_p3_joined : out unsigned((gl_g0 + gl_g0) - 1 downto 0)
  );
end entity gl_m0_split_join;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_split_join is
  signal gl_s1_low : unsigned(gl_g0 - 1 downto 0);
  signal gl_s2_high : unsigned(gl_g0 - 1 downto 0);
  signal gl_s3_joined : unsigned((gl_g0 + gl_g0) - 1 downto 0);
  signal gl_s4_low_internal : unsigned(gl_g0 - 1 downto 0);
  signal gl_s5_high_internal : unsigned(gl_g0 - 1 downto 0);
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
    gl_s4_low_internal <= unsigned(gl_p0_input(((0 + gl_g0) - 1) downto 0));
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s5_high_internal <= unsigned(gl_p0_input(((gl_g0 + gl_g0) - 1) downto gl_g0));
  end process gl_comb_1;
  gl_comb_2 : process(all)
  begin
    gl_s1_low <= gl_s4_low_internal;
  end process gl_comb_2;
  gl_comb_3 : process(all)
  begin
    gl_s2_high <= gl_s5_high_internal;
  end process gl_comb_3;
  gl_comb_4 : process(all)
  begin
    gl_s3_joined <= unsigned((std_logic_vector(gl_s5_high_internal) & std_logic_vector(gl_s4_low_internal)));
  end process gl_comb_4;
  gl_p1_low <= gl_s1_low;
  gl_p2_high <= gl_s2_high;
  gl_p3_joined <= gl_s3_joined;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_split_join_test is
end entity gl_tb0_split_join_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_split_join_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_input : unsigned((8 + 8) - 1 downto 0) := (others => '0');
  signal gl_tb_s1_low : unsigned(7 downto 0);
  signal gl_tb_s2_high : unsigned(7 downto 0);
  signal gl_tb_s3_joined : unsigned((8 + 8) - 1 downto 0);
begin
  gl_dut : entity work.gl_m0_split_join
    generic map (
      gl_g0 => 8
    )
    port map (
      gl_p0_input => gl_tb_s0_input,
      gl_p1_low => gl_tb_s1_low,
      gl_p2_high => gl_tb_s2_high,
      gl_p3_joined => gl_tb_s3_joined
    );
  gl_stimulus : process
  begin
    gl_tb_s0_input <= resize(unsigned'(x"0000000000001234"), 16);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s1_low = resize(unsigned'(x"0000000000000034"), 8))) = '1')
      report "low byte must be 0x34"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s2_high = resize(unsigned'(x"0000000000000012"), 8))) = '1')
      report "high byte must be 0x12"
      severity error;
    assert (gl_bool_to_sl((gl_tb_s3_joined = resize(unsigned'(x"0000000000001234"), 16))) = '1')
      report "concat must restore original word"
      severity error;
    report "GateLisp testbench passed: split-join-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
