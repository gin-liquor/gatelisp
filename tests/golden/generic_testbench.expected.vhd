library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_generic_register is
  generic (
    gl_g0 : positive := 8
  );
  port (
    gl_p0_clk : in std_logic;
    gl_p1_input : in unsigned(gl_g0 - 1 downto 0);
    gl_p2_value : out unsigned(gl_g0 - 1 downto 0)
  );
end entity gl_m0_generic_register;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_generic_register is
  signal gl_s2_value : unsigned(gl_g0 - 1 downto 0);
  signal gl_s3_stored : unsigned(gl_g0 - 1 downto 0) := resize(unsigned'(x"0000000000000000"), gl_g0);
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
    gl_s2_value <= gl_s3_stored;
  end process gl_comb_0;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      gl_s3_stored <= gl_p1_input;
    end if;
  end process gl_seq_0;
  gl_p2_value <= gl_s2_value;
end architecture rtl;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_tb0_generic_register_test is
end entity gl_tb0_generic_register_test;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;
library std;
use std.env.all;

architecture sim of gl_tb0_generic_register_test is
  function gl_bool_to_sl(value : boolean) return std_logic is
  begin
    if value then
      return '1';
    else
      return '0';
    end if;
  end function gl_bool_to_sl;
  signal gl_tb_s0_clk : std_logic := '0';
  signal gl_tb_s1_input : unsigned(15 downto 0) := (others => '0');
  signal gl_tb_s2_value : unsigned(15 downto 0);
  constant gl_clk_period_0 : time := 10 ns;
begin
  gl_dut : entity work.gl_m0_generic_register
    generic map (
      gl_g0 => 16
    )
    port map (
      gl_p0_clk => gl_tb_s0_clk,
      gl_p1_input => gl_tb_s1_input,
      gl_p2_value => gl_tb_s2_value
    );
  gl_clock_0 : process
  begin
    loop
      wait for (gl_clk_period_0 / 2);
      gl_tb_s0_clk <= (not gl_tb_s0_clk);
    end loop;
  end process gl_clock_0;
  gl_stimulus : process
  begin
    gl_tb_s1_input <= resize(unsigned'(x"0000000000001234"), 16);
    wait until rising_edge(gl_tb_s0_clk);
    wait for 1 fs;
    assert (gl_bool_to_sl((gl_tb_s2_value = resize(unsigned'(x"0000000000001234"), 16))) = '1')
      report "generic 16-bit register"
      severity error;
    report "GateLisp testbench passed: generic-register-test" severity note;
    stop;
    wait;
  end process gl_stimulus;
end architecture sim;
