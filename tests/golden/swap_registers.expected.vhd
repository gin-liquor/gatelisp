library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity gl_m0_swap_registers is
  port (
    gl_p0_clk : in std_logic;
    gl_p1_rst : in std_logic;
    gl_p2_a : out unsigned(7 downto 0);
    gl_p3_b : out unsigned(7 downto 0)
  );
end entity gl_m0_swap_registers;

library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

architecture rtl of gl_m0_swap_registers is
  signal gl_s2_a : unsigned(7 downto 0);
  signal gl_s3_b : unsigned(7 downto 0);
  signal gl_s4_reg_a : unsigned(7 downto 0) := resize(unsigned'(x"0000000000000001"), 8);
  signal gl_s5_reg_b : unsigned(7 downto 0) := resize(unsigned'(x"0000000000000002"), 8);
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
    gl_s2_a <= gl_s4_reg_a;
  end process gl_comb_0;
  gl_comb_1 : process(all)
  begin
    gl_s3_b <= gl_s5_reg_b;
  end process gl_comb_1;
  gl_seq_0 : process(gl_p0_clk)
  begin
    if rising_edge(gl_p0_clk) then
      if (gl_p1_rst = '1') then
        gl_s4_reg_a <= resize(unsigned'(x"0000000000000001"), 8);
        gl_s5_reg_b <= resize(unsigned'(x"0000000000000002"), 8);
      else
        gl_s4_reg_a <= gl_s5_reg_b;
        gl_s5_reg_b <= gl_s4_reg_a;
      end if;
    end if;
  end process gl_seq_0;
  gl_p2_a <= gl_s2_a;
  gl_p3_b <= gl_s3_b;
end architecture rtl;
