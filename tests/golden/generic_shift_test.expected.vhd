library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_static_shift is
  generic (
    gl_g0 : natural := 1
  );
  port (
    gl_p0_input : in unsigned((gl_g0 + 1) - 1 downto 0);
    gl_p1_value : out unsigned((gl_g0 + 1) - 1 downto 0)
  );
end entity gl_m0_generic_static_shift;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_static_shift is
  signal gl_s1_value : unsigned((gl_g0 + 1) - 1 downto 0);
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
    gl_s1_value <= shift_left(gl_p0_input, gl_g0);
  end process gl_comb_0;
  gl_p1_value <= gl_s1_value;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_generic_static_shift_test is
end entity gl_tb0_generic_static_shift_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_generic_static_shift_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_input : unsigned((3 + 1) - 1 downto 0) := (others => '0');
  signal gl_tb_s1_value : unsigned((3 + 1) - 1 downto 0);
begin
  gl_dut : entity work.gl_m0_generic_static_shift
    generic map (
      gl_g0 => 3
    )
    port map (
      gl_p0_input => gl_tb_s0_input,
      gl_p1_value => gl_tb_s1_value
    );
  gl_stimulus : process
  begin
    gl_tb_s0_input <= resize(unsigned'(x"0000000000000001"), 4);
    wait for 1 ns;
    assert (gl_bool_to_sl((gl_tb_s1_value = resize(unsigned'(x"0000000000000008"), 4))) = '1')
      report "generic shift"
      severity error;
    report "GateLisp testbench passed: generic-static-shift-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
