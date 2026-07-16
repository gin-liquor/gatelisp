library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_and_gate is
  port (
    gl_p0_a : in std_logic;
    gl_p1_b : in std_logic;
    gl_p2_y : out std_logic
  );
end entity gl_m0_and_gate;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_and_gate is
  signal gl_s2_y : std_logic;
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
    gl_s2_y <= (gl_p0_a and gl_p1_b);
  end process gl_comb_0;
  gl_p2_y <= gl_s2_y;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m1_invert is
  port (
    gl_p3_input : in std_logic;
    gl_p4_output : out std_logic
  );
end entity gl_m1_invert;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m1_invert is
  signal gl_s4_output : std_logic;
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
    gl_s4_output <= (not gl_p3_input);
  end process gl_comb_0;
  gl_p4_output <= gl_s4_output;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m2_hierarchy_top is
  port (
    gl_p5_a : in std_logic;
    gl_p6_b : in std_logic;
    gl_p7_y : out std_logic
  );
end entity gl_m2_hierarchy_top;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m2_hierarchy_top is
  signal gl_s7_y : std_logic;
  signal gl_s8_and_result : std_logic;
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
begin
  gl_i0_and0 : entity work.gl_m0_and_gate
    port map (
      gl_p0_a => gl_p5_a,
      gl_p1_b => gl_p6_b,
      gl_p2_y => gl_s8_and_result
    );
  gl_i1_inv0 : entity work.gl_m1_invert
    port map (
      gl_p3_input => gl_s8_and_result,
      gl_p4_output => gl_s7_y
    );
  gl_p7_y <= gl_s7_y;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_hierarchy_test is
end entity gl_tb0_hierarchy_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_hierarchy_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s5_a : std_logic := '0';
  signal gl_tb_s6_b : std_logic := '0';
  signal gl_tb_s7_y : std_logic;
begin
  gl_dut : entity work.gl_m2_hierarchy_top
    port map (
      gl_p5_a => gl_tb_s5_a,
      gl_p6_b => gl_tb_s6_b,
      gl_p7_y => gl_tb_s7_y
    );
  gl_stimulus : process
  begin
    gl_tb_s5_a <= '0';
    gl_tb_s6_b <= '0';
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s7_y = '1')) = '1')
      report "not (0 and 0)"
      severity error;
    gl_tb_s5_a <= '1';
    gl_tb_s6_b <= '1';
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s7_y = '0')) = '1')
      report "not (1 and 1)"
      severity error;
    report "GateLisp testbench passed: hierarchy-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
